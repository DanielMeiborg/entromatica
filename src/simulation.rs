use std::fmt::Display;

use hashbrown::HashMap;
use petgraph::Graph;

use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Step {
    reachable_states: ReachableStates,
    applied_rules: HashMap<RuleName, Rule>,
}

impl Step {
    pub fn new(reachable_states: ReachableStates, applied_rules: HashMap<RuleName, Rule>) -> Step {
        Step {
            reachable_states,
            applied_rules,
        }
    }
    pub fn reachable_states(&self) -> &ReachableStates {
        &self.reachable_states
    }
    pub fn applied_rules(&self) -> &HashMap<RuleName, Rule> {
        &self.applied_rules
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct History {
    steps: Vec<Step>,
}

impl History {
    pub fn new(reachable_states: ReachableStates) -> History {
        History {
            steps: vec![Step::new(reachable_states, HashMap::new())],
        }
    }
    pub fn steps(&self) -> &Vec<Step> {
        &self.steps
    }
    pub fn time(&self, time: usize) -> Option<&Step> {
        self.steps.get(time)
    }
    pub fn append(&mut self, step: Step) {
        self.steps.push(step);
    }
}

#[derive(Clone, Debug, Default)]
pub struct Simulation {
    history: History,
    rules: HashMap<RuleName, Rule>,
    possible_states: PossibleStates,
    cache: Cache,
}

impl Display for Simulation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Simulation:")?;
        writeln!(f, "  Time: {}", self.time())?;
        writeln!(f, "  Entropy: {}", self.entropy())?;
        writeln!(f, "  Parameters:")?;
        writeln!(f, "  Possible states:")?;
        for (state_hash, state) in self.possible_states.iter() {
            writeln!(f, "    {state_hash}: {state}")?;
        }
        writeln!(f, "  Reachable states:")?;
        for (state_hash, probability) in self.reachable_states().iter() {
            writeln!(f, "    {state_hash}: {probability}")?;
        }
        writeln!(f, "  Rules:")?;
        for (rule_name, rule) in self.rules.iter() {
            writeln!(f, "    {rule_name}: {rule}")?;
        }
        Ok(())
    }
}

impl Iterator for Simulation {
    type Item = Result<Simulation, ErrorKind>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.next_step();
        match result {
            Ok(_) => Some(Ok(self.clone())),
            Err(error) => Some(Err(error)),
        }
    }
}

impl Simulation {
    pub fn new(initial_state: State, rules: HashMap<RuleName, Rule>) -> Simulation {
        let initial_state_hash = StateHash::new(&initial_state);
        Simulation::new_with_reachable_states(
            PossibleStates::from(HashMap::from([(initial_state_hash, initial_state)])),
            ReachableStates::from(HashMap::from([(initial_state_hash, Probability::from(1.))])),
            rules,
        )
    }

    pub fn new_with_reachable_states(
        possible_states: PossibleStates,
        reachable_states: ReachableStates,
        rules: HashMap<RuleName, Rule>,
    ) -> Simulation {
        Simulation {
            possible_states,
            rules,
            history: History::new(reachable_states),
            cache: Cache::new(),
        }
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    pub fn clone_without_history(&self) -> Simulation {
        Simulation {
            history: History::new(self.initial_distribution().clone()),
            rules: self.rules.clone(),
            possible_states: self.possible_states.clone(),
            cache: self.cache.clone(),
        }
    }

    pub fn initial_distribution(&self) -> &ReachableStates {
        self.history.steps().first().unwrap().reachable_states()
    }

    pub fn possible_states(&self) -> &PossibleStates {
        &self.possible_states
    }

    pub fn reachable_states(&self) -> &ReachableStates {
        self.history().steps().last().unwrap().reachable_states()
    }

    pub fn next_step_with_distribution(&mut self, reachable_states: ReachableStates) {
        self.history
            .append(Step::new(reachable_states, HashMap::new()));
    }

    pub fn rules(&self) -> &HashMap<RuleName, Rule> {
        &self.rules
    }

    pub fn time(&self) -> usize {
        self.history().steps().len() - 1
    }

    pub fn entropy(&self) -> Entropy {
        self.reachable_states().entropy()
    }

    pub fn next_step(&mut self) -> Result<(), ErrorKind> {
        let rules = self.rules.clone();
        let next_reachable_states = self.next_reachable_states(&rules)?;
        self.history.append(Step::new(next_reachable_states, rules));
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
        iteration_limit: Option<usize>,
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
        let next_reachable_states = self.next_reachable_states(rules)?;
        self.history
            .append(Step::new(next_reachable_states, rules.clone()));
        Ok(())
    }

    fn next_reachable_states(
        &mut self,
        rules: &HashMap<RuleName, Rule>,
    ) -> Result<ReachableStates, ErrorKind> {
        self.reachable_states().clone().apply_rules(
            &mut self.possible_states,
            &mut self.cache,
            rules,
        )
    }

    pub fn graph(
        &mut self,
        iteration_limit: Option<usize>,
    ) -> Result<Graph<StateHash, RuleName>, ErrorKind> {
        self.full_traversal(iteration_limit, false)?;
        self.cache.graph(self.possible_states.clone())
    }

    pub fn uniform_distribution_is_steady(
        &mut self,
        iteration_limit: Option<usize>,
    ) -> Result<bool, ErrorKind> {
        self.full_traversal(iteration_limit, false)?;
        let simulation = self.clone();
        let uniform_probability = Probability::from(1. / simulation.possible_states.len() as f64);
        let uniform_distribution: ReachableStates = ReachableStates::from(HashMap::from_iter(
            simulation.possible_states.iter().map(|(state_hash, _)| {
                let prob: (StateHash, Probability) = (*state_hash, uniform_probability);
                prob
            }),
        ));
        let mut uniform_simulation = simulation;
        uniform_simulation.next_step_with_distribution(uniform_distribution);
        let uniform_entropy = uniform_simulation.entropy();
        uniform_simulation.next_step()?;
        let uniform_entropy_after_step = uniform_simulation.entropy();
        Ok(uniform_entropy == uniform_entropy_after_step)
    }
}
