use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::mpsc;

#[allow(unused_imports)]
use hashbrown::{HashMap, HashSet};
#[allow(unused_imports)]
use itertools::Itertools;

use derive_more::*;
use petgraph::graph::NodeIndex;
use petgraph::Graph;
use rayon::prelude::*;

#[derive(
    PartialOrd,
    Clone,
    Copy,
    Default,
    Debug,
    Display,
    From,
    Into,
    AsRef,
    AsMut,
    Deref,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Sum,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
)]
pub struct Amount(pub f64);

impl Hash for Amount {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialEq for Amount {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Amount {
    pub fn new() -> Self {
        Self(0.)
    }

    pub fn to_bits(self) -> u64 {
        self.0.to_bits()
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Display, Default)]
pub struct ResourceName(pub String);

impl ResourceName {
    pub fn new() -> Self {
        Self("".to_string())
    }
}

/// A single entity in the simulation.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Entity {
    pub resources: HashMap<ResourceName, Amount>,
}

impl Entity {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn get_resource(&self, resource_name: &ResourceName) -> Amount {
        *self
            .resources
            .get(resource_name)
            .expect("Resource {resource_name} not found")
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Display, Default, From, Into, AsRef, AsMut, Deref)]
pub struct EntityName(pub String);

impl EntityName {
    pub fn new() -> Self {
        Self("".to_string())
    }
}

/// A possible state in the markov chain of the simulation, which is only dependent on
/// the configuration of the entities in the simulation.
#[derive(Clone, Debug, Default)]
pub struct State {
    pub entities: HashMap<EntityName, Entity>,
}

impl Hash for State {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for (name, entity) in &self.entities {
            for (resource_name, amount) in &entity.resources {
                (name, resource_name, amount.to_bits()).hash(state);
            }
        }
    }
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        let self_hasher = &mut DefaultHasher::new();
        self.hash(self_hasher);
        let other_hasher = &mut DefaultHasher::new();
        other.hash(other_hasher);
        self_hasher.finish() == other_hasher.finish()
    }
}

impl Eq for State {}

impl State {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    pub fn get_entity(&self, entity_name: &EntityName) -> Entity {
        self.entities
            .get(entity_name)
            .expect("entity {entity_name} not found")
            .clone()
    }

    pub fn apply_actions(
        &self,
        actions: &Vec<Action>,
        resources: &HashMap<ResourceName, Resource>,
    ) -> State {
        let mut new_state = self.clone();
        for action in actions {
            new_state
                .entities
                .get_mut(&action.entity)
                .expect("Entity {action.entity} not found in state")
                .resources
                .insert(action.resource.clone(), action.new_amount);

            let capacity_per_entity = &resources
                .get(&action.resource)
                .expect("Resource {action.resource} not found in resources")
                .capacity_per_entity;

            if let Capacity::Limited(limit) = capacity_per_entity {
                if action.new_amount > *limit {
                    panic!(
                        "Resource limit per entity exceeded for resource {:#?}",
                        action.resource
                    );
                }
            }
        }
        new_state
    }
}

/// An action a rule can take on a single entity and resource when its condition is met.
#[derive(PartialEq, Clone, Debug, Default)]
pub struct Action {
    // TODO: name to description i.e. vec to hashmap
    pub name: String,
    pub resource: ResourceName,
    pub entity: EntityName,
    pub new_amount: Amount,
}

impl Action {
    pub fn new() -> Self {
        Self {
            name: "".to_string(),
            resource: ResourceName::new(),
            entity: EntityName::new(),
            new_amount: Amount::new(),
        }
    }
}

#[derive(
    PartialOrd,
    Clone,
    Copy,
    Default,
    Debug,
    Display,
    From,
    Into,
    AsRef,
    AsMut,
    Deref,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Sum,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
)]
pub struct ProbabilityWeight(pub f64);

impl Hash for ProbabilityWeight {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialEq for ProbabilityWeight {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl ProbabilityWeight {
    pub fn new() -> Self {
        Self(0.)
    }

    pub fn to_bits(self) -> u64 {
        self.0.to_bits()
    }
}

/// An abstraction over the transition rates of the underlying markov chain.
#[derive(Clone, Debug)]
pub struct Rule {
    pub description: String,

    /// The conditions that must be met for the rule to be applied.
    pub condition: fn(State) -> RuleApplies,

    /// A measure of how often the rule is applied when the condition is met.
    ///
    /// As two rules cannot be applied at the same time, first, the probability that no rule applies is calculated.
    /// The remaining probability is divived among the remaining rules according to their weights.
    pub probability_weight: ProbabilityWeight,

    /// A function which specifies to which state the rule leads when applied.
    ///
    /// The function takes the current state as input and returns multiple actions.
    /// A new state is then created by applying all actions to the current state.
    pub actions: fn(State) -> Vec<Action>,
}

impl Rule {
    #[allow(dead_code)]
    pub fn new(
        description: String,
        condition: fn(State) -> RuleApplies,
        probability_weight: ProbabilityWeight,
        actions: fn(State) -> Vec<Action>,
    ) -> Self {
        Self {
            description,
            condition,
            probability_weight,
            actions,
        }
    }

    pub fn applies(&self, state: &State) -> RuleApplies {
        (self.condition)(state.clone())
    }

    pub fn apply(&self, state: &State, resources: &HashMap<ResourceName, Resource>) -> State {
        let actions = (self.actions)(state.clone());
        state.apply_actions(&actions, resources)
    }
}

#[derive(PartialEq, Clone, Debug)]
#[allow(dead_code)]
pub enum Capacity {
    Limited(Amount),
    Unlimited,
}

impl Capacity {
    pub fn new(limit: Option<Amount>) -> Self {
        match limit {
            Some(amount) => Self::Limited(amount),
            None => Self::Unlimited,
        }
    }
}

/// A resource in the simulation which may or may not have a capacity.
///
/// A resource is essentially a parameter an entity and thus ultimately a state can have.
/// The capacity is a constrain on the amount of the resource being distributed among the entities.
/// It is allowed that the sum of the amounts of a resource among all entities is lesser than the capacity.
/// It is assumed that the capacity is always greater than or equal to zero.
///
/// The capacity_per_entity is an additional constrain on the amount of the resource an individual entity can have.
/// This can again be unlimited.
#[derive(PartialEq, Clone, Debug)]
pub struct Resource {
    pub description: String,
    pub capacity: Capacity,
    pub capacity_per_entity: Capacity,
}

impl Resource {
    pub fn new(description: String, capacity: Capacity, capacity_per_entity: Capacity) -> Self {
        Self {
            description,
            capacity,
            capacity_per_entity,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Display, Default)]
pub struct StateHash(pub u64);

impl StateHash {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn from_state(state: &State) -> Self {
        let mut hasher = &mut DefaultHasher::new();
        state.hash(&mut hasher);
        Self(hasher.finish())
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Display, Default, Not)]
pub struct RuleApplies(pub bool);

impl RuleApplies {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self(false)
    }

    pub fn is_true(&self) -> bool {
        self.0
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Display, Default)]
pub struct RuleName(pub String);

impl RuleName {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self("".to_string())
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
struct RuleCache {
    condition: HashMap<StateHash, RuleApplies>,
    actions: HashMap<StateHash, StateHash>,
}

impl RuleCache {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            condition: HashMap::new(),
            actions: HashMap::new(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
struct Cache {
    pub rules: HashMap<RuleName, RuleCache>,
}

impl Cache {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
struct ConditionCacheUpdate {
    pub rule_name: RuleName,
    pub base_state_hash: StateHash,
    pub applies: RuleApplies,
}

impl ConditionCacheUpdate {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            rule_name: RuleName::new(),
            base_state_hash: StateHash::new(),
            applies: RuleApplies::new(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
struct ActionCacheUpdate {
    pub rule_name: RuleName,
    pub base_state_hash: StateHash,
    pub new_state_hash: StateHash,
}

impl ActionCacheUpdate {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            rule_name: RuleName::new(),
            base_state_hash: StateHash::new(),
            new_state_hash: StateHash::new(),
        }
    }
}

#[derive(
    PartialOrd,
    Clone,
    Copy,
    Default,
    Debug,
    Display,
    From,
    Into,
    AsRef,
    AsMut,
    Deref,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Sum,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
)]
pub struct Entropy(pub f64);

impl Hash for Entropy {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialEq for Entropy {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Entropy {
    pub fn new() -> Self {
        Self(0.)
    }

    pub fn to_bits(self) -> u64 {
        self.0.to_bits()
    }
}

#[derive(
    PartialOrd,
    Clone,
    Copy,
    Default,
    Debug,
    Display,
    From,
    Into,
    AsRef,
    AsMut,
    Deref,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Sum,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
)]
pub struct Probability(f64);

impl Hash for Probability {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialEq for Probability {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Probability {
    pub fn new() -> Self {
        Self(0.)
    }

    pub fn to_bits(self) -> u64 {
        self.0.to_bits()
    }

    pub fn from_probability_weight(probability_weight: ProbabilityWeight) -> Self {
        Self(probability_weight.0)
    }
}

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Copy,
    Default,
    Debug,
    Display,
    From,
    Into,
    AsRef,
    AsMut,
    Deref,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Sum,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
)]
pub struct Time(i64);

impl Time {
    pub fn new() -> Self {
        Self(0)
    }
}
#[derive(Clone, PartialEq, Eq, Debug, Default, From, Into, AsRef, AsMut, Index, Deref)]
pub struct PossibleStates(HashMap<StateHash, State>);

impl PossibleStates {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn append_state(&mut self, state_hash: StateHash, state: State) {
        self.0.insert(state_hash, state);
    }

    pub fn append_states(&mut self, states: &PossibleStates) {
        for (state_hash, state) in states.0.iter() {
            self.append_state(*state_hash, state.clone());
        }
    }

    pub fn state(&self, state_hash: &StateHash) -> Option<State> {
        self.0.get(state_hash).cloned()
    }

    pub fn keys(&self) -> std::iter::Cloned<hashbrown::hash_map::Keys<StateHash, State>> {
        self.0.keys().cloned()
    }

    pub fn values(&self) -> std::iter::Cloned<hashbrown::hash_map::Values<StateHash, State>> {
        self.0.values().cloned()
    }
}

#[derive(Clone, PartialEq, Debug, Default, From, Into, AsRef, AsMut, Index, Deref)]
pub struct ReachableStates(HashMap<StateHash, Probability>);

impl ReachableStates {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn append_state(&mut self, state_hash: StateHash, state_probability: Probability) {
        match self.0.get_mut(&state_hash) {
            Some(probability) => {
                *probability += state_probability;
            }
            None => {
                self.0.insert(state_hash, state_probability);
            }
        }
    }

    pub fn append_states(&mut self, states: &ReachableStates) {
        for (state_hash, state_probability) in states.iter() {
            self.append_state(*state_hash, *state_probability);
        }
    }

    pub fn keys(&self) -> std::iter::Cloned<hashbrown::hash_map::Keys<StateHash, Probability>> {
        self.0.keys().cloned()
    }

    pub fn values(&self) -> std::iter::Cloned<hashbrown::hash_map::Values<StateHash, Probability>> {
        self.0.values().cloned()
    }

    pub fn probability_sum(&self) -> Probability {
        Probability(self.par_iter().map(|(_, probability)| probability.0).sum())
    }
}

/// All information and methods needed to run the simulation.
///
/// All information is managed by the methods of this struct.
/// Do not change properties manually.
#[derive(Clone, Debug, Default)]
pub struct Simulation {
    /// All resources in the simulation.
    ///
    /// The key is the name of the resource, while the value the resource itself.
    /// This must not change after initialization.
    pub resources: HashMap<ResourceName, Resource>,

    /// The initial state of the simulation.
    ///
    /// This state has a starting probability of 1.
    /// This must not change after initialization.
    pub initial_state: State,

    /// All states which are possible at at some point during the simulation.
    ///
    /// The key is the hash of the state, while the value is the state itself.
    pub possible_states: PossibleStates,

    /// All states which are possible at the current timestep.
    ///
    /// The key is the hash of the state, while the value is the probability that this state occurs.
    pub reachable_states: ReachableStates,

    /// All rules in the simulation.
    ///
    /// This must not change after initialization.
    pub rules: HashMap<RuleName, Rule>,

    /// The current timestep of the simulation, starting at 0.
    pub time: Time,

    /// The current entropy of the probability distribution of the reachable_states.
    pub entropy: Entropy,

    /// The cache used for performance purposes.
    cache: Cache,
}

impl Simulation {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            initial_state: State::new(),
            possible_states: PossibleStates::new(),
            reachable_states: ReachableStates::new(),
            rules: HashMap::new(),
            time: Time::new(),
            entropy: Entropy::new(),
            cache: Cache::new(),
        }
    }

    /// Creates a new simulation with the given resources, initial state and rules.
    pub fn create(
        resources: HashMap<ResourceName, Resource>,
        initial_state: State,
        rules: HashMap<RuleName, Rule>,
    ) -> Simulation {
        let initial_state_hash = StateHash::from_state(&initial_state);

        let rule_caches: HashMap<RuleName, RuleCache> = rules
            .par_iter()
            .map(|(name, _)| {
                (
                    name.clone(),
                    RuleCache {
                        condition: HashMap::new(),
                        actions: HashMap::new(),
                    },
                )
            })
            .collect();

        Simulation {
            resources,
            initial_state: initial_state.clone(),
            possible_states: PossibleStates(HashMap::from([(initial_state_hash, initial_state)])),
            reachable_states: ReachableStates(HashMap::from([(
                initial_state_hash,
                Probability(1.),
            )])),
            rules,
            time: Time(0),
            entropy: Entropy(0.),
            cache: Cache { rules: rule_caches },
        }
    }

    /// Runs the simulation for one timestep.
    pub fn next_step(&mut self) {
        self.update_reachable_states();
        self.entropy = self.get_entropy();
        self.time += Time(1);
    }

    /// Checks if the given state satisfies all resource constrains.
    fn check_resource_capacities(&self, new_state: &State) {
        for (resource_name, resource) in &self.resources {
            match &resource.capacity {
                Capacity::Limited(limit) => {
                    let mut total_amount = Amount(0.);
                    for (entity_name, entity) in &new_state.entities {
                        let entity_amount = entity
                            .resources
                            .get(resource_name)
                            .expect("Entity {entity_name} does not have resource {resource_name}");
                        if *entity_amount < Amount(0.) {
                            panic!(
                                "Entity {} has negative amount of resource {}",
                                entity_name, resource_name
                            );
                        }
                        total_amount += *entity_amount;
                        if total_amount > *limit {
                            panic!(
                                "Resource limit exceeded for resource {resource_name}",
                                resource_name = resource_name
                            );
                        }
                    }
                }
                Capacity::Unlimited => {
                    for (entity_name, entity) in &new_state.entities {
                        let entity_amount = entity
                            .resources
                            .get(resource_name)
                            .expect("Entity {entity_name} does not have resource {resource_name}");
                        if *entity_amount < Amount(0.) {
                            panic!(
                                "Entity {} has negative amount of resource {}",
                                entity_name, resource_name
                            );
                        }
                    }
                }
            }
        }
    }

    /// Checks if a given rule applies to the given state using or updating the cache respectively.
    fn check_if_rule_applies(
        &self,
        rule_name: &RuleName,
        state_hash: &StateHash,
    ) -> (RuleApplies, Option<ConditionCacheUpdate>) {
        let rule_cache = self
            .cache
            .rules
            .get(rule_name)
            .expect("Rule {rule_name} not found in cache");
        let rule = self
            .rules
            .get(rule_name)
            .expect("Rule {rule_name} not found");
        if rule.probability_weight == ProbabilityWeight(0.) {
            return (RuleApplies(false), None);
        }
        match rule_cache.condition.get(state_hash) {
            Some(rule_applies) => (*rule_applies, None),
            None => {
                let state = self
                    .possible_states
                    .state(state_hash)
                    .expect("State with hash {state_hash} not found in possible_states");
                let result = rule.applies(&state);
                let cache = ConditionCacheUpdate {
                    rule_name: rule_name.clone(),
                    base_state_hash: *state_hash,
                    applies: result,
                };
                (result, Some(cache))
            }
        }
    }

    /// Gets the state the given rule results in from the given state using or updating the cache respectively.
    fn get_new_state(
        &self,
        base_state_hash: &StateHash,
        rule_name: &RuleName,
    ) -> (State, Option<ActionCacheUpdate>) {
        let rule_cache = self
            .cache
            .rules
            .get(rule_name)
            .expect("Rule {rule_name} not found in cache");

        if let Some(state_hash) = rule_cache.actions.get(base_state_hash) {
            if let Some(new_state) = self.possible_states.state(state_hash) {
                return (new_state, None);
            }
        }

        let rule = self
            .rules
            .get(rule_name)
            .expect("Rule {rule_name} not found");
        let base_state = self
            .possible_states
            .state(base_state_hash)
            .expect("Base state {base_state_hash} not found in possible_states");
        let new_state = rule.apply(&base_state, &self.resources);

        self.check_resource_capacities(&new_state);

        let new_state_hash = StateHash::from_state(&new_state);
        let cache_update = ActionCacheUpdate {
            rule_name: rule_name.clone(),
            base_state_hash: *base_state_hash,
            new_state_hash,
        };
        (new_state, Some(cache_update))
    }

    // Add all reachable states from the base state to reachable_states and possible_states while using or updating the cache respectively.
    fn get_reachable_states_from_base_state(
        &self,
        base_state_hash: &StateHash,
        base_state_probability: &Probability,
    ) -> (
        ReachableStates,
        PossibleStates,
        Vec<ConditionCacheUpdate>,
        Vec<ActionCacheUpdate>,
    ) {
        let mut new_base_state_probability: Probability = *base_state_probability;
        let mut applying_rules_probability_weight_sum = ProbabilityWeight(0.);
        let mut reachable_states_from_base_state_by_rule_probability_weight: HashMap<
            StateHash,
            ProbabilityWeight,
        > = HashMap::new();

        let mut condition_cache_updates = Vec::new();
        let mut action_cache_updates = Vec::new();

        let mut new_possible_states: PossibleStates = PossibleStates::new();

        for (rule_name, rule) in &self.rules {
            let (rule_applies, condition_cache_update) =
                self.check_if_rule_applies(rule_name, base_state_hash);
            if let Some(cache) = condition_cache_update {
                condition_cache_updates.push(cache);
            }
            if rule_applies.is_true() {
                new_base_state_probability *= 1. - f64::from(rule.probability_weight);
                applying_rules_probability_weight_sum += rule.probability_weight;
                let (new_state, action_cache_update) =
                    self.get_new_state(base_state_hash, rule_name);
                if let Some(cache) = action_cache_update {
                    action_cache_updates.push(cache);
                }
                let new_state_hash = StateHash::from_state(&new_state);
                new_possible_states.append_state(new_state_hash, new_state);
                reachable_states_from_base_state_by_rule_probability_weight
                    .insert(new_state_hash, rule.probability_weight);
            }
        }

        let mut new_reachable_states = ReachableStates::new();

        if new_base_state_probability > Probability(0.) {
            new_reachable_states.append_state(*base_state_hash, new_base_state_probability);
        }

        let probabilities_for_reachable_states_from_base_state =
            Simulation::get_probabilities_for_reachable_states_from_base_state(
                reachable_states_from_base_state_by_rule_probability_weight,
                *base_state_hash,
                *base_state_probability,
                new_base_state_probability,
                applying_rules_probability_weight_sum,
            );
        probabilities_for_reachable_states_from_base_state
            .iter()
            .for_each(|(new_state_hash, new_state_probability)| {
                new_reachable_states.append_state(*new_state_hash, *new_state_probability);
            });
        (
            new_reachable_states,
            new_possible_states,
            condition_cache_updates,
            action_cache_updates,
        )
    }

    fn get_probabilities_for_reachable_states_from_base_state(
        reachable_states_from_base_state_by_rule_probability_weight: HashMap<
            StateHash,
            ProbabilityWeight,
        >,
        base_state_hash: StateHash,
        base_state_probability: Probability,
        new_base_state_probability: Probability,
        applying_rules_probability_weight_sum: ProbabilityWeight,
    ) -> ReachableStates {
        ReachableStates(HashMap::from_par_iter(
            reachable_states_from_base_state_by_rule_probability_weight
                .par_iter()
                .filter_map(|(new_reachable_state_hash, rule_probability_weight)| {
                    if *new_reachable_state_hash != base_state_hash {
                        let new_reachable_state_probability =
                            Probability::from_probability_weight(*rule_probability_weight)
                                * f64::from(base_state_probability)
                                * f64::from(Probability(1.) - new_base_state_probability)
                                / f64::from(applying_rules_probability_weight_sum);
                        Option::Some((*new_reachable_state_hash, new_reachable_state_probability))
                    } else {
                        Option::None
                    }
                }),
        ))
    }

    // TODO: Reimplement multithreading
    /// Update reachable_states and possible_states to the next time step.
    fn update_reachable_states(&mut self) {
        let (condition_cache_updates_tx, condition_cache_updates_rx) = mpsc::channel();
        let (action_cache_updates_tx, action_cache_updates_rx) = mpsc::channel();

        let old_reachable_states = self.reachable_states.clone();
        self.reachable_states = ReachableStates::new();
        old_reachable_states
            .iter()
            .for_each(|(base_state_hash, base_state_probability)| {
                let (
                    new_reachable_states,
                    new_possible_states,
                    condition_cache_updates,
                    action_cache_update,
                ) = self
                    .get_reachable_states_from_base_state(base_state_hash, base_state_probability);
                for cache_update in condition_cache_updates {
                    condition_cache_updates_tx.send(cache_update).unwrap();
                }
                for cache_update in action_cache_update {
                    action_cache_updates_tx.send(cache_update).unwrap();
                }
                self.possible_states.append_states(&new_possible_states);
                self.reachable_states.append_states(&new_reachable_states);
            });

        // TODO: Assert that the cache does not yet contain the cache update
        while let Result::Ok(condition_cache_update) = condition_cache_updates_rx.try_recv() {
            let own_rule_cache = self
                .cache
                .rules
                .get_mut(&condition_cache_update.rule_name)
                .expect("Rule {rule_name} not found in self.cache");
            own_rule_cache.condition.insert(
                condition_cache_update.base_state_hash,
                condition_cache_update.applies,
            );
        }

        while let Result::Ok(action_cache_update) = action_cache_updates_rx.try_recv() {
            let own_rule_cache = self
                .cache
                .rules
                .get_mut(&action_cache_update.rule_name)
                .expect("Rule {rule_name} not found in self.cache");
            own_rule_cache.actions.insert(
                action_cache_update.base_state_hash,
                action_cache_update.new_state_hash,
            );
        }

        // TODO: Improve this
        let probability_sum = self.reachable_states.probability_sum();
        if !(Probability(0.9999999) < probability_sum && probability_sum < Probability(1.0000001)) {
            panic!("Probability sum {:?} is not 1", probability_sum);
        }
    }

    /// Gets the entropy of the current probability distribution.
    fn get_entropy(&self) -> Entropy {
        Entropy(
            self.reachable_states
                .0
                .par_iter()
                .map(|(_, probability)| {
                    if *probability > Probability(0.) {
                        f64::from(*probability) * -f64::from(*probability).log2()
                    } else {
                        0.
                    }
                })
                .sum(),
        )
    }

    ///Gets a graph from the possible states with the nodes being the states and the directed edges being the rule names.
    pub fn get_graph_from_cache(&self) -> Graph<State, RuleName> {
        let mut graph = Graph::<State, RuleName>::new();
        let mut nodes: HashMap<StateHash, NodeIndex> = HashMap::new();
        for (state_hash, state) in &self.possible_states.0 {
            let node_index = graph.add_node(state.clone());
            nodes.insert(*state_hash, node_index);
        }
        for (state_hash, state_node) in &nodes {
            for (rule_name, rule_cache) in &self.cache.rules {
                if rule_cache.condition.get(state_hash).is_some() {
                    if let Some(new_state_hash) = rule_cache.actions.get(state_hash) {
                        let new_state_node = nodes.get(new_state_hash).unwrap();
                        graph.add_edge(*state_node, *new_state_node, rule_name.clone());
                    }
                }
            }
        }
        graph
    }

    /// Checks if the uniform distribution is a steady state i.e. if the transition rate matrix is doubly statistical.
    pub fn is_doubly_statistical(&self) -> bool {
        let mut simulation = Simulation::create(
            self.resources.clone(),
            self.initial_state.clone(),
            self.rules.clone(),
        );
        let mut current_reachable_states = simulation.reachable_states.clone();
        while current_reachable_states.len() != self.reachable_states.len()
            && current_reachable_states
                .keys()
                .all(|state_hash| self.reachable_states.contains_key(&state_hash))
        {
            current_reachable_states = simulation.reachable_states.clone();
            simulation.next_step();
        }
        let uniform_probability = Probability(1. / simulation.possible_states.len() as f64);
        let uniform_distribution: ReachableStates = ReachableStates(HashMap::from_iter(
            simulation.possible_states.iter().map(|(state_hash, _)| {
                let prob: (StateHash, Probability) = (*state_hash, uniform_probability);
                prob
            }),
        ));
        let mut uniform_simulation = simulation.clone();
        uniform_simulation.reachable_states = uniform_distribution;
        let uniform_entropy = uniform_simulation.get_entropy();
        uniform_simulation.next_step();
        let uniform_entropy_after_step = uniform_simulation.get_entropy();
        uniform_entropy == uniform_entropy_after_step
    }
}
