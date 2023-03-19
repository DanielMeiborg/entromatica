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

/// The key part of the rule-mechanism.
///
/// A rule consists of four parts:
/// - A condition, which determines whether the rule applies to a given state.
/// - A weight, which determines the relative probability of the rule if the
///   condition applies. This must be a value between 0 and 1.
/// - An action, which determines the new state if the rule applies.
/// - A description, which is used for as the description of the transition.
///
/// The generic parameter `T` is the state itself.
///
/// For each transition, only a single rule can be applied. If no rule applies,
/// the state remains the same. If exactly one rule applies, the probability
/// weight is the probability for that transition. The following mechanism is
/// used to determine the probabilities for the new states if multiple rules
/// apply to the same state:
///
/// 1. The probability of **no rule** applying is calculated by multiplying for
///    all rules 1 - the rule's weight.
/// 2. The remaining probability is distributed among the rules according to the
///    weights.
///
/// So if two rules apply for the same state, if both have a weight of 1 (which
/// normally would mean that the rule applies with 100% probability), the
/// probability for each transition is 0.5. If both rules have a weight of 0.5
/// instead, the probability for the do-nothing transition is 0.25, and the
/// probability for each rules transition is 0.375.
///
///
/// # Example
/// ```rust
/// use entromatica::prelude::*;
/// use entromatica::models::rules::*;
/// use std::sync::Arc;
/// use hashbrown::HashMap;
///
/// // A random walk where the chance of suddenly returning to the initial state is 0.1
/// let initial_state = 0;
/// let return_rule: Rule<i32> = Rule::new(
///     "Return".to_string(),
///     Arc::new(|state| state != 0),
///     0.1,
///     Arc::new(|_| 0),
/// );
/// let forward_rule: Rule<i32> = Rule::new(
///     "Forward".to_string(),
///     Arc::new(|_| true),
///     1.,
///     Arc::new(|state| state + 1),
/// );
///
/// let backward_rule: Rule<i32> = Rule::new(
///     "Backward".to_string(),
///     Arc::new(|_| true),
///     1.,
///     Arc::new(|state| state - 1),
/// );
///
/// let rules = vec![forward_rule, backward_rule, return_rule];
///
/// let state_transition_generator = get_state_transition_generator(rules);
/// let mut simulation = Simulation::new(initial_state, state_transition_generator);
///
/// // state == 0
/// assert_eq!(simulation.probability_distribution(0).len(), 1);
///
/// // now -1 and 1 are equally likely
/// simulation.next_step();
/// assert_eq!(simulation.probability_distribution(1).len(), 2);
///
/// // now are -2, 0 and 2 possible
/// simulation.next_step();
/// assert_eq!(simulation.probability_distribution(2).len(), 3);
///
/// // and last but not least -3, -1, 0, 1 and 3. 0 is only possible because of the return rule
/// simulation.next_step();
/// assert_eq!(simulation.probability_distribution(3).len(), 5);
/// ```
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
    /// Create a new rule.
    ///
    /// # Arguments
    /// - `description`: A description of the rule. This is used for the
    ///   description of the transition.
    /// - `condition`: A function that determines whether the rule applies to a
    ///   given state.
    /// - `probability_weight`: The probability weight of the rule. This is used
    ///   to calculate the probability of the transition.
    /// - `action`: A function that determines the new state if the rule
    ///   applies.
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

    /// Executes the rule's condition function on the given state and returns
    /// the result.
    pub fn applies(&self, state: T) -> RuleApplies {
        (self.condition)(state)
    }

    /// Executes the rule's action function on the given state and returns the result.
    pub fn apply(&self, state: T) -> T {
        (self.action)(state)
    }

    /// Returns the rule's probability weight.
    pub fn weight(&self) -> ProbabilityWeight {
        self.weight
    }

    /// Returns the rule's description.
    pub fn description(&self) -> &String {
        &self.description
    }

    /// Returns a reference to the rule's condition function.
    pub fn condition(&self) -> &(dyn Fn(T) -> RuleApplies + Send + Sync) {
        &*self.condition
    }

    /// Returns a reference to the rule's action function.
    pub fn action(&self) -> &(dyn Fn(T) -> T + Send + Sync) {
        &*self.action
    }
}

/// A function that creates a state transition generator from a set of rules.
///
/// # Arguments
/// - `rules`: A list of rules that are used to create the state transition
///  generator.
///
/// # Returns
/// A state transition generator that can be used to create a simulation.
pub fn get_state_transition_generator<T>(rules: Vec<Rule<T>>) -> StateTransitionGenerator<T, String>
where
    T: Debug + Clone + Send + Sync + 'static + PartialEq + Eq + Hash,
{
    Arc::new(move |state: T| -> OutgoingTransitions<T, String> {
        let new_states_by_weight = rules
            .iter()
            .filter(|rule| rule.applies(state.clone()))
            .map(|rule| {
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

        let rules = vec![forward_rule, backward_rule];

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

        let rules = vec![forward_rule, backward_rule, return_rule];

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
