use std::fmt::Display;

#[allow(unused_imports)]
use hashbrown::{HashMap, HashSet};
#[allow(unused_imports)]
use itertools::Itertools;
#[allow(unused_imports)]
use rayon::prelude::*;

use petgraph::Graph;

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

impl Display for Simulation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Simulation:")?;
        writeln!(f, "  Time: {}", self.time)?;
        writeln!(f, "  Entropy: {}", self.entropy)?;
        writeln!(f, "  Resources:")?;
        for (resource_name, resource) in self.resources.iter() {
            writeln!(f, "    {}: {}", resource_name, resource)?;
        }
        writeln!(f, "  Initial state:")?;
        writeln!(f, "{}", self.initial_state)?;
        writeln!(f, "  Possible states:")?;
        for (state_hash, state) in self.possible_states.iter() {
            writeln!(f, "    {}: {}", state_hash, state)?;
        }
        writeln!(f, "  Reachable states:")?;
        for (state_hash, probability) in self.reachable_states.iter() {
            writeln!(f, "    {}: {}", state_hash, probability)?;
        }
        writeln!(f, "  Rules:")?;
        for (rule_name, rule) in self.rules.iter() {
            writeln!(f, "    {}: {}", rule_name, rule)?;
        }
        Ok(())
    }
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
    ) -> Result<Simulation, ErrorKind> {
        let initial_state_hash = StateHash::from_state(&initial_state);
        for (_, entity) in initial_state.iter_entities() {
            for (resource_name, _) in entity.iter_resources() {
                if !resources.contains_key(resource_name) {
                    return Err(ErrorKind::ResourceNotFound(NotFoundError::new(
                        resource_name.clone(),
                        entity.clone(),
                    )));
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
    pub fn next_step(&mut self) -> Result<(), ErrorKind> {
        self.reachable_states.apply_rules(
            &mut self.possible_states,
            &mut self.cache,
            &self.resources,
            &self.rules,
        )?;
        self.entropy = self.reachable_states.entropy();
        self.time.increment();
        Ok(())
    }

    pub fn run(&mut self, steps: usize) -> Result<(), ErrorKind> {
        for _ in 0..steps {
            self.next_step()?;
        }
        Ok(())
    }

    pub fn update_reachable_states(
        &mut self,
        rules: HashMap<RuleName, Rule>,
    ) -> Result<(), ErrorKind> {
        self.reachable_states.apply_rules(
            &mut self.possible_states,
            &mut self.cache,
            &self.resources,
            &rules,
        )
    }

    ///Gets a graph from the possible states with the nodes being the states and the directed edges being the rule names.
    pub fn graph(&self) -> Graph<State, RuleName> {
        self.cache.graph(self.possible_states.clone())
    }

    /// Checks if the uniform distribution is a steady state i.e. if the transition rate matrix is doubly statistical.
    pub fn uniform_distribution_is_steady(&self) -> Result<bool, ErrorKind> {
        let mut simulation = Simulation::from(
            self.resources.clone(),
            self.initial_state.clone(),
            self.rules.clone(),
        )?;
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
