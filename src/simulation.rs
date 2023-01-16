use std::fmt::Display;

use hashbrown::HashMap;
use petgraph::Graph;

use crate::prelude::*;

#[derive(Clone, Debug, Default)]
pub struct Simulation {
    initial_state: State,
    possible_states: PossibleStates,
    reachable_states: ReachableStates,
    rules: HashMap<RuleName, Rule>,
    time: Time,
    entropy: Entropy,
    cache: Cache,
}

impl Display for Simulation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Simulation:")?;
        writeln!(f, "  Time: {}", self.time)?;
        writeln!(f, "  Entropy: {}", self.entropy)?;
        writeln!(f, "  Parameters:")?;
        writeln!(f, "  Initial state:")?;
        writeln!(f, "{}", self.initial_state)?;
        writeln!(f, "  Possible states:")?;
        for (state_hash, state) in self.possible_states.iter() {
            writeln!(f, "    {state_hash}: {state}")?;
        }
        writeln!(f, "  Reachable states:")?;
        for (state_hash, probability) in self.reachable_states.iter() {
            writeln!(f, "    {state_hash}: {probability}")?;
        }
        writeln!(f, "  Rules:")?;
        for (rule_name, rule) in self.rules.iter() {
            writeln!(f, "    {rule_name}: {rule}")?;
        }
        Ok(())
    }
}

impl Simulation {
    pub fn new(initial_state: State, rules: HashMap<RuleName, Rule>) -> Simulation {
        let initial_state_hash = StateHash::new(&initial_state);
        Simulation {
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
        }
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

    pub fn next_step(&mut self) -> Result<(), ErrorKind> {
        let rules = self.rules.clone();
        self.update_reachable_states(&rules)?;
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

    /// Performs a full traversal of the possible states of the system.
    ///
    /// This method will continue to call `next_step()` until all possible states have been visited.
    /// If an `iteration_limit` is provided, the traversal will stop if the time spent exceeds the limit.
    /// If modify_state is set to false, only the cache and the possible states will be updated,
    /// but the simulation will otherwise remain at its current state.
    ///
    /// # Errors
    ///
    /// - `ErrorKind::IterationLimitReached` - If the traversal took longer than the provided iteration limit.
    ///   Note that any progress will be  applied to the simulation.
    /// ```
    pub fn full_traversal(
        &mut self,
        iteration_limit: Option<Time>,
        modify_state: bool,
    ) -> Result<(), ErrorKind> {
        if modify_state {
            let mut num_current_possible_states = 0;
            while num_current_possible_states != self.possible_states().len() {
                if let Some(iteration_limit) = iteration_limit {
                    if self.time() >= iteration_limit {
                        return Err(ErrorKind::IterationLimitReached {
                            time: self.time(),
                            context: get_backtrace(),
                        });
                    }
                }
                num_current_possible_states = self.possible_states().len();
                self.next_step()?;
            }
            Ok(())
        } else {
            let mut simulation_clone = self.clone();
            simulation_clone.full_traversal(iteration_limit, true)?;
            self.possible_states
                .merge(simulation_clone.possible_states())?;
            self.cache.merge(&simulation_clone.cache)?;
            Ok(())
        }
    }

    pub fn apply_intervention(&mut self, rules: &HashMap<RuleName, Rule>) -> Result<(), ErrorKind> {
        for rule_name in rules.keys() {
            if self.rules.contains_key(rule_name) {
                return Err(ErrorKind::from(CacheError::RuleAlreadyExists {
                    rule_name: rule_name.clone(),
                    context: get_backtrace(),
                }));
            }
        }
        self.update_reachable_states(rules)?;
        self.entropy = self.reachable_states.entropy();
        self.time.increment();
        Ok(())
    }

    fn update_reachable_states(
        &mut self,
        rules: &HashMap<RuleName, Rule>,
    ) -> Result<(), ErrorKind> {
        self.reachable_states
            .apply_rules(&mut self.possible_states, &mut self.cache, rules)
    }

    pub fn graph(&self) -> Graph<State, RuleName> {
        self.cache.graph(self.possible_states.clone())
    }


    pub fn uniform_distribution_is_steady(
        &mut self,
        iteration_limit: Option<Time>,
    ) -> Result<bool, ErrorKind> {
        let mut simulation = Simulation::new(self.initial_state.clone(), self.rules.clone());
        simulation.full_traversal(iteration_limit, false)?;
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
