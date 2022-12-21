use std::sync::mpsc;

#[allow(unused_imports)]
use hashbrown::{HashMap, HashSet};
#[allow(unused_imports)]
use itertools::Itertools;

use petgraph::Graph;
use rayon::prelude::*;

pub mod state;
use state::*;

pub mod resource;
use resource::*;

pub mod units;
use units::*;

pub mod rules;
use rules::*;

mod cache;
use cache::*;

pub mod error;
#[allow(unused_imports)]
use error::*;

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
    resources: HashMap<ResourceName, Resource>,

    /// The initial state of the simulation.
    ///
    /// This state has a starting probability of 1.
    /// This must not change after initialization.
    initial_state: State,

    /// All states which are possible at at some point during the simulation.
    ///
    /// The key is the hash of the state, while the value is the state itself.
    possible_states: PossibleStates,

    /// All states which are possible at the current timestep.
    ///
    /// The key is the hash of the state, while the value is the probability that this state occurs.
    reachable_states: ReachableStates,

    /// All rules in the simulation.
    ///
    /// This must not change after initialization.
    rules: HashMap<RuleName, Rule>,

    /// The current timestep of the simulation, starting at 0.
    time: Time,

    /// The current entropy of the probability distribution of the reachable_states.
    entropy: Entropy,

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
    pub fn from(
        resources: HashMap<ResourceName, Resource>,
        initial_state: State,
        rules: HashMap<RuleName, Rule>,
    ) -> Result<Simulation, NotFoundError<ResourceName, (EntityName, Entity)>> {
        let initial_state_hash = StateHash::from_state(&initial_state);
        for (entity_name, entity) in initial_state.iter_entities() {
            for (resource_name, _) in entity.iter_resources() {
                if !resources.contains_key(resource_name) {
                    return Err(NotFoundError::new(
                        resource_name.clone(),
                        (entity_name.clone(), entity.clone()),
                    ));
                }
            }
        }

        Ok(Simulation {
            resources,
            initial_state: initial_state.clone(),
            possible_states: PossibleStates::from(HashMap::from([(
                initial_state_hash,
                initial_state,
            )])),
            reachable_states: ReachableStates::from(HashMap::from([(
                initial_state_hash,
                Probability::from(1.),
            )])),
            rules,
            time: Time::from(0),
            entropy: Entropy::from(0.),
            cache: Cache::new(),
        })
    }

    pub fn resources(&self) -> &HashMap<ResourceName, Resource> {
        &self.resources
    }

    pub fn initial_state(&self) -> &State {
        &self.initial_state
    }

    pub fn possible_states(&self) -> &PossibleStates {
        &self.possible_states
    }

    pub fn reachable_states(&self) -> &ReachableStates {
        &self.reachable_states
    }

    pub fn rules(&self) -> &HashMap<RuleName, Rule> {
        &self.rules
    }

    pub fn time(&self) -> Time {
        self.time
    }

    pub fn entropy(&self) -> Entropy {
        self.entropy
    }

    /// Runs the simulation for one timestep.
    pub fn next_step(&mut self) -> Result<(), ResourceCapacityError> {
        self.update_reachable_states()?;
        self.entropy = self.reachable_states.entropy();
        self.time.increment();
        Ok(())
    }

    pub fn run(&mut self, steps: usize) -> Result<(), ResourceCapacityError> {
        for _ in 0..steps {
            self.next_step()?;
        }
        Ok(())
    }

    // Add all reachable states from the base state to reachable_states and possible_states while using or updating the cache respectively.
    fn reachable_states_from_base_state(
        &self,
        base_state_hash: &StateHash,
        base_state_probability: &Probability,
    ) -> Result<
        (
            ReachableStates,
            PossibleStates,
            Vec<ConditionCacheUpdate>,
            Vec<ActionCacheUpdate>,
        ),
        ResourceCapacityError,
    > {
        let mut new_base_state_probability: Probability = *base_state_probability;
        let mut applying_rules_probability_weight_sum = ProbabilityWeight::from(0.);
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
                rule.applies(&self.cache, rule_name.clone(), state.clone());
            if let Some(cache) = condition_cache_update {
                condition_cache_updates.push(cache);
            }
            if rule_applies.is_true() {
                new_base_state_probability *= 1. - f64::from(rule.weight());
                applying_rules_probability_weight_sum += rule.weight();
                let base_state = self
                    .possible_states
                    .state(base_state_hash)
                    .expect("Base state not found in possible_states");
                let (new_state, action_cache_update) = rule.apply(
                    &self.cache,
                    &self.possible_states,
                    rule_name.clone(),
                    *base_state_hash,
                    base_state.clone(),
                    &self.resources,
                )?;
                if let Some(cache) = action_cache_update {
                    action_cache_updates.push(cache);
                }
                let new_state_hash = StateHash::from_state(&new_state);
                new_possible_states
                    .append_state(new_state_hash, new_state)
                    .expect("State {state_hash} already exists in possible_states");
                reachable_states_from_base_state_by_rule_probability_weight
                    .insert(new_state_hash, rule.weight());
            }
        }

        let mut new_reachable_states = ReachableStates::new();

        if new_base_state_probability > Probability::from(0.) {
            new_reachable_states
                .append_state(*base_state_hash, new_base_state_probability)
                .unwrap();
        }

        let probabilities_for_reachable_states_from_base_state =
            Simulation::probabilities_for_reachable_states(
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
        Ok((
            new_reachable_states,
            new_possible_states,
            condition_cache_updates,
            action_cache_updates,
        ))
    }

    fn probabilities_for_reachable_states(
        reachable_states_by_rule_probability_weight: HashMap<StateHash, ProbabilityWeight>,
        base_state_hash: StateHash,
        base_state_probability: Probability,
        new_base_state_probability: Probability,
        applying_rules_probability_weight_sum: ProbabilityWeight,
    ) -> ReachableStates {
        ReachableStates::from(HashMap::from_par_iter(
            reachable_states_by_rule_probability_weight
                .par_iter()
                .filter_map(|(new_reachable_state_hash, rule_probability_weight)| {
                    if *new_reachable_state_hash != base_state_hash {
                        let new_reachable_state_probability =
                            Probability::from_probability_weight(*rule_probability_weight)
                                * f64::from(base_state_probability)
                                * f64::from(Probability::from(1.) - new_base_state_probability)
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
    fn update_reachable_states(&mut self) -> Result<(), ResourceCapacityError> {
        let (condition_cache_updates_tx, condition_cache_updates_rx) = mpsc::channel();
        let (action_cache_updates_tx, action_cache_updates_rx) = mpsc::channel();

        let old_reachable_states = self.reachable_states.clone();
        self.reachable_states = ReachableStates::new();
        for (base_state_hash, base_state_probability) in old_reachable_states.iter() {
            let (
                new_reachable_states,
                new_possible_states,
                condition_cache_updates,
                action_cache_update,
            ) = self.reachable_states_from_base_state(base_state_hash, base_state_probability)?;
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
        }

        while let Result::Ok(condition_cache_update) = condition_cache_updates_rx.try_recv() {
            self.cache
                .apply_condition_update(condition_cache_update)
                .unwrap();
        }

        while let Result::Ok(action_cache_update) = action_cache_updates_rx.try_recv() {
            self.cache.apply_action_update(action_cache_update).unwrap();
        }
        debug_assert!(
            !(Probability::from(0.9999999) < self.reachable_states.probability_sum()
                && self.reachable_states.probability_sum() < Probability::from(1.0000001))
        );
        Ok(())
    }

    ///Gets a graph from the possible states with the nodes being the states and the directed edges being the rule names.
    pub fn graph(&self) -> Graph<State, RuleName> {
        self.cache.graph(self.possible_states.clone())
    }

    /// Checks if the uniform distribution is a steady state i.e. if the transition rate matrix is doubly statistical.
    pub fn uniform_distribution_is_steady(&self) -> Result<bool, ResourceCapacityError> {
        let mut simulation = Simulation::from(
            self.resources.clone(),
            self.initial_state.clone(),
            self.rules.clone(),
        )
        .map_err(ResourceCapacityError::NotFound)?;
        let mut current_reachable_states = simulation.reachable_states.clone();
        while current_reachable_states.len() != self.reachable_states.len()
            && current_reachable_states
                .iter()
                .map(|(state_hash, _)| state_hash)
                .all(|state_hash| self.reachable_states.contains(state_hash))
        {
            current_reachable_states = simulation.reachable_states.clone();
            simulation.next_step()?;
        }
        let uniform_probability = Probability::from(1. / simulation.possible_states.len() as f64);
        let uniform_distribution: ReachableStates = ReachableStates::from(HashMap::from_iter(
            simulation.possible_states.iter().map(|(state_hash, _)| {
                let prob: (StateHash, Probability) = (*state_hash, uniform_probability);
                prob
            }),
        ));
        let mut uniform_simulation = simulation.clone();
        uniform_simulation.reachable_states = uniform_distribution;
        let uniform_entropy = uniform_simulation.reachable_states.entropy();
        uniform_simulation.next_step()?;
        let uniform_entropy_after_step = uniform_simulation.reachable_states.entropy();
        Ok(uniform_entropy == uniform_entropy_after_step)
    }
}

// TODO: Add tests
