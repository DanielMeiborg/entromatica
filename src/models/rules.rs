use std::{fmt::Display, sync::Arc};

use derive_more::{From, Into};
use hashbrown::HashMap;
use itertools::Itertools;
use std::fmt::Debug;
use std::hash::Hash;

use crate::prelude::*;

pub type EntityName = String;
pub type ParameterName = String;
pub type Entity<T> = HashMap<ParameterName, T>;

pub type RuleName = String;
pub type RuleApplies = bool;
pub type ProbabilityWeight = f64;

#[derive(From, Into, Clone)]
pub struct Rule<T> {
    description: String,
    condition: Arc<dyn Fn(T) -> RuleApplies + Send + Sync>,
    weight: ProbabilityWeight,
    action: Arc<dyn Fn(T) -> T + Send + Sync>,
}

impl<T: Debug> Debug for Rule<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Rule:")?;
        writeln!(f, "Description: {}", self.description)?;
        writeln!(f, "Weight: {}", self.weight)?;
        Ok(())
    }
}

impl<T> Display for Rule<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Rule:")?;
        writeln!(f, "Description: {}", self.description)?;
        writeln!(f, "Weight: {}", self.weight)?;
        Ok(())
    }
}

impl<T> Rule<T> {
    pub fn new(
        description: String,
        condition: Arc<dyn Fn(T) -> RuleApplies + Send + Sync>,
        probability_weight: ProbabilityWeight,
        action: Arc<dyn Fn(T) -> T + Send + Sync>,
    ) -> Self {
        Self {
            description,
            condition,
            weight: probability_weight,
            action,
        }
    }

    pub fn applies(&self, state: T) -> RuleApplies {
        (self.condition)(state)
    }

    pub fn apply(&self, state: T) -> T {
        (self.action)(state)
    }

    pub fn weight(&self) -> ProbabilityWeight {
        self.weight
    }

    pub fn description(&self) -> &String {
        &self.description
    }

    pub fn condition(&self) -> &(dyn Fn(T) -> RuleApplies + Send + Sync) {
        &*self.condition
    }

    pub fn action(&self) -> &(dyn Fn(T) -> T + Send + Sync) {
        &*self.action
    }
}

pub fn get_state_transition_generator<T>(
    rules: HashMap<RuleName, Rule<T>>,
) -> StateTransitionGenerator<T, String>
where
    T: Debug + Clone + Send + Sync + 'static + PartialEq + Eq + Hash,
{
    Arc::new(move |state: T| -> OutgoingTransitions<T, String> {
        let new_states_by_weight = rules
            .iter()
            .filter(|(_, rule)| rule.applies(state.clone()))
            .map(|(_, rule)| {
                let new_state: T = rule.apply(state.clone());
                let weight = rule.weight();
                let description = rule.description().clone();
                (hash(&new_state), (new_state, weight, description))
            })
            .fold(
                HashMap::new(),
                |acc: HashMap<u64, (T, ProbabilityWeight, String)>,
                 (_, (state, weight, description))| {
                    let mut new_acc = acc;
                    if let Some(e) = new_acc.get_mut(&hash(&state)) {
                        e.1 += weight;
                        e.2 = format!("{} | {}", e.2, description);
                    } else {
                        new_acc.insert(hash(&state), (state.clone(), weight, description));
                    }
                    new_acc
                },
            );
        let base_state_hash = hash(&state);
        let nothing_probability = new_states_by_weight
            .iter()
            .map(|(_, (_, weight, _))| 1. - *weight)
            .product::<ProbabilityWeight>();
        let weight_sum = new_states_by_weight
            .iter()
            .map(|(_, (_, weight, _))| weight)
            .sum::<ProbabilityWeight>()
            + nothing_probability;
        let mut new_states = new_states_by_weight
            .into_iter()
            .map(|(state_hash, (state, weight, description))| {
                (state_hash, (state, weight / weight_sum, description))
            })
            .collect::<HashMap<u64, (T, f64, String)>>();
        if nothing_probability > 0. {
            new_states
                .entry(base_state_hash)
                .and_modify(|(_, probability, description)| {
                    *probability += nothing_probability / weight_sum;
                    description.push_str(" | Nothing");
                })
                .or_insert((
                    state,
                    nothing_probability / weight_sum,
                    "Nothing".to_string(),
                ));
        }
        new_states
            .into_iter()
            .map(|(_, (state, probability, description))| (state, description, probability))
            .collect_vec()
    }) as StateTransitionGenerator<T, String>
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_walk() {
        let initial_state = 0;

        let forward_rule: Rule<i32> = Rule::new(
            "Forward".to_string(),
            Arc::new(|_| true),
            1.,
            Arc::new(|state| state + 1),
        );

        let backward_rule: Rule<i32> = Rule::new(
            "Backward".to_string(),
            Arc::new(|_| true),
            1.,
            Arc::new(|state| state - 1),
        );

        let rules: HashMap<RuleName, Rule<i32>> = HashMap::from([
            ("forward".to_string(), forward_rule),
            ("backward".to_string(), backward_rule),
        ]);

        let state_transition_generator = get_state_transition_generator(rules);
        let mut simulation = Simulation::new(initial_state, state_transition_generator);
        dbg!(&simulation);
        assert_eq!(simulation.known_states().len(), 1);
        assert_eq!(simulation.known_transitions().len(), 0);
        assert_eq!(simulation.probability_distributions().len(), 1);
        assert_eq!(simulation.state_transition_graph().node_count(), 1);
        assert_eq!(simulation.state_transition_graph().edge_count(), 0);
        assert_eq!(simulation.entropy(0), 0.0);

        simulation.next_step();
        dbg!(&simulation);
        assert_eq!(simulation.known_states().len(), 3);
        assert_eq!(simulation.known_transitions().len(), 2);
        assert_eq!(simulation.probability_distributions().len(), 2);
        assert_eq!(simulation.state_transition_graph().node_count(), 3);
        assert_eq!(simulation.state_transition_graph().edge_count(), 2);
        assert_eq!(simulation.entropy(1), 1.0);

        let graph = simulation.state_transition_graph();
        dbg!(&graph);
        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);

        assert_eq!(simulation.state_probability(0, 1), 0.);
        assert_eq!(simulation.state_probability(1, 1), 0.5);
        assert_eq!(simulation.state_probability(-1, 1), 0.5);

        assert_eq!(simulation.initial_distribution(), HashMap::from([(0, 1.0)]));

        assert_eq!(simulation.time(), 1);
    }

    #[test]
    fn random_walk_return() {
        let initial_state = 0;
        let return_rule: Rule<i32> = Rule::new(
            "Return".to_string(),
            Arc::new(|_| true),
            0.1,
            Arc::new(|_| 0),
        );
        let forward_rule: Rule<i32> = Rule::new(
            "Forward".to_string(),
            Arc::new(|_| true),
            1.,
            Arc::new(|state| state + 1),
        );

        let backward_rule: Rule<i32> = Rule::new(
            "Backward".to_string(),
            Arc::new(|_| true),
            1.,
            Arc::new(|state| state - 1),
        );

        let rules: HashMap<RuleName, Rule<i32>> = HashMap::from([
            ("forward".to_string(), forward_rule),
            ("backward".to_string(), backward_rule),
            ("return".to_string(), return_rule),
        ]);

        let state_transition_generator = get_state_transition_generator(rules);
        let mut simulation = Simulation::new(initial_state, state_transition_generator);
        dbg!(&simulation);
        assert_eq!(simulation.known_states().len(), 1);
        assert_eq!(simulation.known_transitions().len(), 0);
        assert_eq!(simulation.probability_distributions().len(), 1);
        assert_eq!(simulation.state_transition_graph().node_count(), 1);
        assert_eq!(simulation.state_transition_graph().edge_count(), 0);
        assert_eq!(simulation.entropy(0), 0.0);

        simulation.next_step();
        dbg!(&simulation);
        assert_eq!(simulation.known_states().len(), 3);
        assert_eq!(dbg!(simulation.known_transitions()).len(), 3);
        assert_eq!(simulation.probability_distributions().len(), 2);
        assert_eq!(simulation.state_transition_graph().node_count(), 3);
        assert_eq!(simulation.state_transition_graph().edge_count(), 3);
        dbg!(simulation.entropy(1));
    }
}
