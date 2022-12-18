use std::sync::mpsc;

#[allow(unused_imports)]
use hashbrown::{HashMap, HashSet};
#[allow(unused_imports)]
use itertools::Itertools;

use petgraph::graph::NodeIndex;
use petgraph::Graph;
use rayon::prelude::*;

pub mod state;
use state::*;

pub mod resources;
use resources::*;

pub mod units;
use units::*;

pub mod rules;
use rules::*;

mod cache;
use cache::*;

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
        self.entropy = self.reachable_states.entropy();
        self.time += Time(1);
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
            let state = self
                .possible_states
                .state(base_state_hash)
                .expect("Base state {base_state_hash} not found in possible_states");
            let (rule_applies, condition_cache_update) =
                rule.applies_using_cache(&self.cache, rule_name.clone(), state);
            if let Some(cache) = condition_cache_update {
                condition_cache_updates.push(cache);
            }
            if rule_applies.is_true() {
                new_base_state_probability *= 1. - f64::from(rule.probability_weight);
                applying_rules_probability_weight_sum += rule.probability_weight;
                let base_state = self
                    .possible_states
                    .state(base_state_hash)
                    .expect("Base state not found in possible_states");
                let (new_state, action_cache_update) = rule.apply_using_cache(
                    &self.cache,
                    &self.possible_states,
                    rule_name.clone(),
                    *base_state_hash,
                    base_state,
                    &self.resources,
                );
                if let Some(cache) = action_cache_update {
                    action_cache_updates.push(cache);
                }
                let new_state_hash = StateHash::from_state(&new_state);
                new_possible_states
                    .append_state(new_state_hash, new_state)
                    .expect("State {state_hash} already exists in possible_states");
                reachable_states_from_base_state_by_rule_probability_weight
                    .insert(new_state_hash, rule.probability_weight);
            }
        }

        let mut new_reachable_states = ReachableStates::new();

        if new_base_state_probability > Probability(0.) {
            new_reachable_states
                .append_state(*base_state_hash, new_base_state_probability)
                .unwrap();
        }

        let probabilities_for_reachable_states_from_base_state =
            Simulation::get_probabilities_for_reachable_states(
                reachable_states_from_base_state_by_rule_probability_weight,
                *base_state_hash,
                *base_state_probability,
                new_base_state_probability,
                applying_rules_probability_weight_sum,
            );
        probabilities_for_reachable_states_from_base_state
            .iter()
            .for_each(|(new_state_hash, new_state_probability)| {
                new_reachable_states
                    .append_state(*new_state_hash, *new_state_probability)
                    .unwrap();
            });
        (
            new_reachable_states,
            new_possible_states,
            condition_cache_updates,
            action_cache_updates,
        )
    }

    fn get_probabilities_for_reachable_states(
        reachable_states_by_rule_probability_weight: HashMap<StateHash, ProbabilityWeight>,
        base_state_hash: StateHash,
        base_state_probability: Probability,
        new_base_state_probability: Probability,
        applying_rules_probability_weight_sum: ProbabilityWeight,
    ) -> ReachableStates {
        ReachableStates(HashMap::from_par_iter(
            reachable_states_by_rule_probability_weight
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
                self.possible_states
                    .append_states(&new_possible_states)
                    .expect("Possible states already exist");
                self.reachable_states
                    .append_states(&new_reachable_states)
                    .unwrap();
            });

        while let Result::Ok(condition_cache_update) = condition_cache_updates_rx.try_recv() {
            let own_rule_cache = self
                .cache
                .rules
                .get_mut(&condition_cache_update.rule_name)
                .expect("Rule {rule_name} not found in self.cache");
            if let Some(update) = own_rule_cache
                .condition
                .get(&condition_cache_update.base_state_hash)
            {
                if update != &condition_cache_update.applies {
                    panic!("Condition cache update already exists");
                }
            }
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
            if let Some(update) = own_rule_cache
                .actions
                .get(&action_cache_update.base_state_hash)
            {
                if update != &action_cache_update.new_state_hash {
                    panic!("Action cache update already exists");
                }
            }
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
                .all(|state_hash| self.reachable_states.contains_key(state_hash))
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
        let uniform_entropy = uniform_simulation.reachable_states.entropy();
        uniform_simulation.next_step();
        let uniform_entropy_after_step = uniform_simulation.reachable_states.entropy();
        uniform_entropy == uniform_entropy_after_step
    }
}
