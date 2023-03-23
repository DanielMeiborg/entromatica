use std::{
    fmt::Debug,
    hash::Hash,
    sync::{Arc, Mutex},
};

use crate::prelude::*;
use hashbrown::HashMap;
use itertools::Itertools;
use ndarray::Array2;
use petgraph::{graph::Graph, visit::EdgeRef};
use rayon::prelude::*;

type StateHash = u64;
type KnownStates<S> = HashMap<StateHash, S>;

type TransitionHash = u64;
type KnownTransitions<T> = HashMap<TransitionHash, T>;

type StateTransitionGraph = Graph<StateHash, (TransitionHash, Probability)>;

pub type StateTransitionGenerator<S, T> =
    Arc<dyn Fn(S) -> OutgoingTransitions<S, T> + Send + Sync + 'static>;

pub type StateProbabilityDistribution<S> = HashMap<S, Probability>;

pub type OutgoingTransitions<S, T> = Vec<(S, T, Probability)>;

type HashedStateProbabilityDistribution = HashMap<StateHash, Probability>;

pub type Probability = f64;
pub type Time = u64;

/// `Simulation` is the a struct for a cached markov chain simulation.
///
/// `Simulation` has two generic parameters:
/// - `S`: The type of the states in the markov chain.
/// - `T`: The type of the transitions in the markov chain, usually a
/// description. To do anything useful both have to be `Hash + Clone + Send +
/// Sync + PartialEq + Eq + Debug`.
///
/// It primarily consists of an initial state `S` and a
/// [StateTransitionGenerator](type.StateTransitionGenerator.html). This
/// generator is a function that takes a state and returns a list of the next
/// states in the markov chain with their respective relative probabilities.
///
/// This function must be fully deterministic, as this generator is cached, so
/// it is only called once for each state anyway.
///
/// # Example
///
/// ```rust
/// // This is a simple onedimensional random walk
/// use entromatica::prelude::*;
/// use std::sync::Arc;
///
/// // The initial state. It has to be Hash + Clone + Send + Sync + PartialEq + Eq + Debug
/// let initial_state: i32 = 0;
///
/// // The state transition generator. The simulation panics if the probabilities don't sum to 1.0
/// let state_transition_generator =
/// Arc::new(|state: i32| vec![(state + 1, "next", 0.5), (state - 1, "previous", 0.5)]);
///
/// let mut simulation = Simulation::new(initial_state, state_transition_generator);
///
/// // The Shannon-entropy at the given time
/// assert_eq!(simulation.entropy(0), 0.0);
/// simulation.next_step();
/// assert_eq!(simulation.entropy(1), 1.0);
/// ```
#[derive(Clone)]
pub struct Simulation<S, T> {
    state_transition_graph: StateTransitionGraph,
    probability_distributions: HashMap<Time, HashedStateProbabilityDistribution>,
    known_states: KnownStates<S>,
    known_transitions: KnownTransitions<T>,
    state_transition_generator: CachedFunction<S, OutgoingTransitions<S, T>>,
}

impl<S, T> Debug for Simulation<S, T>
where
    S: Hash + Clone + Send + Sync + PartialEq + Debug,
    T: Hash + Clone + Send + Sync + PartialEq + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Simulation")
            .field("state_transition_graph", &self.state_transition_graph)
            .field("probabilities", &self.probability_distributions)
            .field("known_states", &self.known_states)
            .field("known_transitions", &self.known_transitions)
            .finish()
    }
}

impl<S, T> Simulation<S, T>
where
    S: Hash + Clone + Send + Sync + PartialEq + Eq + Debug,
    T: Hash + Clone + Send + Sync + PartialEq + Eq + Debug,
{
    /// Create a new `Simulation` with the given initial state and state transition generator.
    ///
    /// As the initial distribution this results in a single state with a probability of 1.0.
    pub fn new(
        initial_state: S,
        state_transition_generator: StateTransitionGenerator<S, T>,
    ) -> Self {
        let initial_state_hash = hash(&initial_state);

        let mut state_transition_graph = Graph::new();
        state_transition_graph.add_node(initial_state_hash);

        let probabilities = HashMap::from([(0, HashMap::from([(initial_state_hash, 1.0)]))]);

        let known_states = HashMap::from([(initial_state_hash, initial_state)]);

        let known_transitions = HashMap::new();

        Self {
            state_transition_graph,
            probability_distributions: probabilities,
            known_states,
            known_transitions,
            state_transition_generator: CachedFunction::new(state_transition_generator),
        }
    }

    /// Create a new `Simulation` with the given initial state distribution and
    /// state transition generator.
    ///
    /// The initial state distribution is a `HashMap` from states to their
    /// respective probabilities.
    pub fn new_with_distribution(
        probabilities: StateProbabilityDistribution<S>,
        state_transition_generator: StateTransitionGenerator<S, T>,
    ) -> Self {
        let known_states = probabilities
            .iter()
            .map(|(state, _)| {
                let state_hash = hash(state);
                (state_hash, state.clone())
            })
            .collect::<HashMap<_, _>>();

        let known_transitions = HashMap::new();

        let hashed_probabilities = probabilities
            .iter()
            .map(|(state, probability)| {
                let state_hash = hash(state);
                (state_hash, *probability)
            })
            .collect::<HashMap<_, _>>();

        let mut graph: StateTransitionGraph = Graph::new();
        probabilities.iter().for_each(|(state, _)| {
            let state_hash = hash(state);
            graph.add_node(state_hash);
        });

        Self {
            state_transition_graph: graph,
            probability_distributions: HashMap::from([(0, hashed_probabilities)]),
            known_states,
            known_transitions,
            state_transition_generator: CachedFunction::new(state_transition_generator),
        }
    }

    fn state(&self, state_hash: StateHash) -> Option<&S> {
        self.known_states.get(&state_hash)
    }

    fn transition(&self, transition_hash: TransitionHash) -> Option<&T> {
        self.known_transitions.get(&transition_hash)
    }

    /// The state transitioning graph of the markov chain.
    ///
    /// The nodes of the graph are the states of the markov chain and the edges
    /// a tuple of the transition and the probability of the transition.
    ///
    /// The graph is directed, so the direction of the edge indicates the
    /// direction of the transition.
    ///
    /// This method uses all information that is available to the markov chain
    /// regardless of the current timeline. This means that the graph will
    /// include nodes and transitions generated by e.g. the
    /// (full_traversal)[#method.full_traversal] method.
    pub fn state_transition_graph(&self) -> Graph<S, (T, Probability)> {
        let mut graph = Graph::new();
        self.state_transition_graph
            .node_indices()
            .map(|node| {
                let state_hash = *self.state_transition_graph.node_weight(node).unwrap();
                self.state(state_hash).unwrap().clone()
            })
            .for_each(|state| {
                graph.add_node(state);
            });
        let edges = self
            .state_transition_graph
            .edge_references()
            .map(|edge| {
                let source_hash = *self
                    .state_transition_graph
                    .node_weight(edge.source())
                    .unwrap();
                let target_hash = *self
                    .state_transition_graph
                    .node_weight(edge.target())
                    .unwrap();
                let (transition_hash, probability) = edge.weight();
                let source = self.state(source_hash).unwrap().clone();
                let target = self.state(target_hash).unwrap().clone();
                let transition = self.transition(*transition_hash).unwrap().clone();
                (source, target, transition, *probability)
            })
            .collect::<Vec<(S, S, T, Probability)>>();
        for (source_state, target_state, transition, probability) in edges {
            // check if nodes already exist
            let source = graph
                .node_indices()
                .find(|node| graph.node_weight(*node).unwrap() == &source_state)
                .unwrap_or_else(|| graph.add_node(source_state.clone()));
            let target = graph
                .node_indices()
                .find(|node| graph.node_weight(*node).unwrap() == &target_state)
                .unwrap_or_else(|| graph.add_node(target_state.clone()));
            graph.update_edge(source, target, (transition, probability));
        }
        graph
    }

    /// Get a HashMap of the probability distributions indexed by time.
    ///
    /// Each probability distribution is a HashMap from states to their
    /// probabilities. The time starts at zero and increases by one for each
    /// step.
    pub fn probability_distributions(&self) -> HashMap<Time, StateProbabilityDistribution<S>> {
        self.probability_distributions
            .iter()
            .map(|(time, state_probability_distribution)| {
                let state_probability_distribution = state_probability_distribution
                    .iter()
                    .map(|(state_hash, probability)| {
                        let state = self.state(*state_hash).unwrap().clone();
                        (state, *probability)
                    })
                    .collect::<HashMap<_, _>>();

                (*time, state_probability_distribution)
            })
            .collect::<HashMap<_, _>>()
    }

    /// Get the probability of a specific state for the given time.
    ///
    /// If the state is not known at the given time, the probability is zero.
    pub fn state_probability(&self, state: S, time: Time) -> f64 {
        self.probability_distributions
            .get(&time)
            .and_then(|state_probability_distribution| {
                state_probability_distribution.get(&hash(&state))
            })
            .copied()
            .unwrap_or(0.0)
    }

    /// Get the probability distribution for the initial distribution.
    pub fn initial_distribution(&self) -> StateProbabilityDistribution<S> {
        self.probability_distribution(0)
    }

    /// Get the probability distribution for the given time.
    ///
    /// If the time is not known, the method panics.
    pub fn probability_distribution(&self, time: Time) -> StateProbabilityDistribution<S> {
        self.probability_distributions
            .get(&time)
            .map(|state_probability_distribution| {
                state_probability_distribution
                    .iter()
                    .map(|(state_hash, probability)| {
                        let state = self.state(*state_hash).unwrap().clone();
                        (state, *probability)
                    })
                    .collect::<HashMap<_, _>>()
            })
            .expect("No probability distribution found for given time")
    }

    /// Gets a list of all known states.
    ///
    /// States are known when they have been returned at some point by the state
    /// transition generator. The ordering is arbitrary, not necessarily
    /// consistent over multiple calls and can change at any time in the future.
    pub fn known_states(&self) -> Vec<S> {
        self.known_states.values().cloned().collect()
    }

    /// Gets a list of all known transitions.
    ///
    /// Transitions are known when they have been returned at some point by the
    /// state transition generator. The ordering is arbitrary, not necessarily
    /// consistent over multiple calls and can change at any time in the future.
    pub fn known_transitions(&self) -> Vec<T> {
        self.known_transitions.values().cloned().collect()
    }

    /// Get the shannon entropy of the markov chain at the given time.
    pub fn entropy(&self, time: Time) -> f64 {
        let state_probability_distribution = self.probability_distribution(time);
        let entropy = state_probability_distribution
            .values()
            .map(|probability| probability * probability.log2())
            .sum::<f64>()
            .abs();
        entropy
    }

    /// Get the current time of the markov chain.
    ///
    /// The time starts at zero and increases by one for each step. This method
    /// returns the time of the newest probability distribution.
    pub fn time(&self) -> Time {
        self.probability_distributions
            .keys()
            .max()
            .copied()
            .unwrap_or(0)
    }

    /// Update the markov chain by one step.
    ///
    /// This method returns the new probability distribution. This method calls
    /// the state transition generator for each state in the current probability
    /// distribution. These probabilities are then multiplied with the
    /// probability of the state they are called on. The new probability
    /// distribution is the combination of all those distributions.
    ///
    /// # Panics
    /// This method panics if the probabilities of the state transition
    /// generator do not sum up to 1.0.
    pub fn next_step(&mut self) -> StateProbabilityDistribution<S> {
        let initial_time = self.time();
        let state_probability_distribution: Vec<(S, Probability)> = self
            .probability_distribution(initial_time)
            .into_par_iter()
            .collect();

        let state_transition_probabilities = self.state_transition_generator.call_many_parallel(
            state_probability_distribution
                .par_iter()
                .map(|(state, _)| state.clone()),
        );

        // Check if probabilities sum up to 1.0
        state_transition_probabilities
            .par_iter()
            .for_each(|next_states| {
                assert_eq!(
                    (next_states
                        .iter()
                        .map(|(_, _, probability)| probability)
                        .sum::<Probability>()
                        * 10_i64.pow(10) as f64)
                        .round()
                        / 10_i64.pow(10) as f64,
                    1.0,
                    "Sum of probabilities of next states is not 1.0"
                );
            });

        // Calculate new state probability distribution
        let new_hashed_state_probability_distribution_mutex = Mutex::new(HashMap::new());
        state_transition_probabilities
            .par_iter()
            .zip_eq(state_probability_distribution.par_iter())
            .for_each(|(next_states, (_, current_state_probability))| {
                next_states.iter().for_each(|(new_state, _, probability)| {
                    new_hashed_state_probability_distribution_mutex
                        .lock()
                        .unwrap()
                        .entry(hash(new_state))
                        .and_modify(|state_probability| {
                            *state_probability += current_state_probability * probability;
                        })
                        .or_insert(current_state_probability * probability);
                });
            });
        // Add new state probability distribution to list of all state probability distributions
        self.probability_distributions.insert(
            initial_time + 1,
            new_hashed_state_probability_distribution_mutex
                .into_inner()
                .unwrap(),
        );

        // Add new states and transitions to known states and transitions
        state_transition_probabilities
            .iter()
            .for_each(|next_states| {
                next_states.iter().for_each(|(new_state, transition, _)| {
                    self.known_states.insert(hash(new_state), new_state.clone());
                    self.known_transitions
                        .insert(hash(transition), transition.clone());
                });
            });

        // Add new state transitions to state transition graph
        state_transition_probabilities
            .iter()
            .zip(state_probability_distribution.iter())
            .for_each(|(next_states, (old_state, _))| {
                next_states
                    .iter()
                    .for_each(|(new_state, transition, probability)| {
                        let source = self
                            .state_transition_graph
                            .node_indices()
                            .find(|node_index| {
                                self.state_transition_graph
                                    .node_weight(*node_index)
                                    .unwrap()
                                    == &hash(old_state)
                            })
                            .unwrap();
                        let target = self
                            .state_transition_graph
                            .node_indices()
                            .find(|node_index| {
                                self.state_transition_graph
                                    .node_weight(*node_index)
                                    .unwrap()
                                    == &hash(new_state)
                            })
                            .unwrap_or_else(|| {
                                self.state_transition_graph.add_node(hash(new_state))
                            });
                        self.state_transition_graph.update_edge(
                            source,
                            target,
                            (hash(transition), *probability),
                        );
                    });
            });

        // Return the new state probability distribution
        self.probability_distribution(initial_time + 1)
    }

    /// Update the markov chain until all states are known.
    ///
    /// This method calls the (next_step)[#method.next_step] method until the
    /// list of known states does not change anymore. This means that all states
    /// reachable by the markov chain are then traversed. If the
    /// modify_cache_only property is set to `true` the list of probability
    /// distributions is not updated. Things like
    /// [state_transition_graph](#method.state_transition_graph) or
    /// [known_states](#method.known_states) will still be affected by this
    /// traversal.
    ///
    /// If the number of states is infite this method will never return.
    pub fn full_traversal(&mut self, modify_cache_only: bool) {
        if modify_cache_only {
            let mut simulation_clone = self.clone();
            let mut num_current_known_states = 0;
            while num_current_known_states != simulation_clone.known_states.len() {
                num_current_known_states = simulation_clone.known_states.len();
                simulation_clone.next_step();
                self.known_states = simulation_clone.known_states.clone();
                self.known_transitions = simulation_clone.known_transitions.clone();
                self.state_transition_graph = simulation_clone.state_transition_graph.clone();
                self.state_transition_generator =
                    simulation_clone.state_transition_generator.clone();
            }
        } else {
            let mut num_current_known_states = 0;
            while num_current_known_states != self.known_states.len() {
                num_current_known_states = self.known_states.len();
                self.next_step();
            }
        }
    }

    /// Check if the uniform distribution is steady.
    ///
    /// This method checks if the uniform distribution is stable i.e. if it
    /// doesn't change anymore if this distribution is set.
    ///
    /// If the number of states is infinte this method will never return. It
    /// will modify the cache of the markov chain, so e.g.
    /// [state_transition_graph](#method.state_transition_graph) will afterwards
    /// show the full markov chain.
    pub fn uniform_distribution_is_steady(&mut self) -> bool {
        self.full_traversal(true);
        let mut simulation_clone = self.clone();
        let uniform_probability = 1.0 / self.known_states.len() as Probability;
        let uniform_state_probability_distribution = self
            .known_states
            .iter()
            .map(|(state_hash, _)| (*state_hash, uniform_probability))
            .collect::<HashMap<_, _>>();
        let next_time = simulation_clone.time() + 1;
        simulation_clone
            .probability_distributions
            .insert(next_time, uniform_state_probability_distribution);
        let uniform_entropy = simulation_clone.entropy(next_time);
        simulation_clone.next_step();
        let uniform_entropy_after_step = simulation_clone.entropy(next_time + 1);
        uniform_entropy_after_step == uniform_entropy
    }

    /// Get the transition rate matrix of the markov chain.
    ///
    /// This method returns the transition rate matrix of the state transition
    /// Graph. This is a ndarray where the value at index (i, j) is the
    /// probability that the markov chain transitions from state i to state j.
    /// To do that it makes a cache-only full traversal.
    ///
    /// The second part of the return type is the ordering. If a state is at the
    /// nth position in the vector, it means that it corresponds to the nth row
    /// and the nth column. The ordering is arbitrary, not necessarily
    /// consistent over multiple calls and can change at any time in the future.
    ///
    /// If the number of states is infinte this method will never return.
    ///
    /// # Example
    /// ```rust
    ///  use entromatica::prelude::*;
    ///  use std::sync::Arc;
    ///
    /// // A simple random walk in a circle with NUM_STATES positions
    /// let initial_state = 0;
    /// const NUM_STATES: i32 = 5;
    /// let state_transition_generator = Arc::new(|state: i32| -> OutgoingTransitions<i32, &str> {
    ///     vec![
    ///         (
    ///             {
    ///                 if state + 1 == NUM_STATES {
    ///                     0
    ///                 } else {
    ///                     state + 1
    ///                 }
    ///             },
    ///             "forward",
    ///             0.5,
    ///         ),
    ///         (
    ///             {
    ///                 if state - 1 == -1 {
    ///                     NUM_STATES - 1
    ///                 } else {
    ///                     state - 1
    ///                 }
    ///             },
    ///             "backward",
    ///             0.5,
    ///         ),
    ///     ]
    /// });
    /// let mut simulation = Simulation::new(initial_state, state_transition_generator);
    /// let (transition_rate_matrix, ordering) = simulation.transition_rate_matrix();
    /// // The transition rate matrix is a square matrix with a size equal to the number of states
    /// assert_eq!(transition_rate_matrix.nrows(), NUM_STATES as usize);
    /// assert_eq!(transition_rate_matrix.ncols(), NUM_STATES as usize);
    ///
    /// // The probability of transitioning from state 0 to 1 is 0.5
    /// let index = (ordering.iter().position(|state| *state == 0).unwrap(), ordering.iter().position(|state| *state == 1).unwrap());
    /// assert_eq!(transition_rate_matrix.get(index), Some(&0.5));
    /// ```
    pub fn transition_rate_matrix(&mut self) -> (Array2<Probability>, Vec<S>) {
        self.full_traversal(true);
        let ordering_hash_map: HashMap<StateHash, usize> = self
            .known_states
            .iter()
            .enumerate()
            .map(|(index, (hash, _))| (*hash, index))
            .collect();
        let mut transition_rate_matrix =
            Array2::zeros((ordering_hash_map.len(), ordering_hash_map.len()));
        self.state_transition_graph
            .edge_references()
            .for_each(|edge_reference| {
                let source_index = ordering_hash_map
                    .get(
                        self.state_transition_graph
                            .node_weight(edge_reference.source())
                            .unwrap(),
                    )
                    .unwrap();
                let target_index = ordering_hash_map
                    .get(
                        self.state_transition_graph
                            .node_weight(edge_reference.target())
                            .unwrap(),
                    )
                    .unwrap();
                *transition_rate_matrix
                    .get_mut((*source_index, *target_index))
                    .unwrap() = edge_reference.weight().1;
            });
        (
            transition_rate_matrix,
            ordering_hash_map
                .iter()
                .map(|(hash, order)| (order, self.known_states.get(hash).unwrap()))
                .sorted_by(|(index_a, _), (index_b, _)| index_a.cmp(index_b))
                .map(|(_, state)| state.clone())
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use ndarray::{Array1, Axis};

    use super::*;

    #[test]
    fn random_walk() {
        let initial_state = 0;
        let state_transition_generator =
            Arc::new(|state: i32| vec![(state + 1, "next", 0.5), (state - 1, "previous", 0.5)]);

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
    fn random_walk_with_initial_distribution() {
        let initial_distribution = HashMap::from([(0, 0.5), (1, 0.5)]);
        let state_transition_generator =
            Arc::new(|state: i32| vec![(state + 1, "next", 0.5), (state - 1, "previous", 0.5)]);
        let mut simulation =
            Simulation::new_with_distribution(initial_distribution, state_transition_generator);
        assert_eq!(simulation.known_states().len(), 2);
        assert_eq!(simulation.known_transitions().len(), 0);
        assert_eq!(simulation.probability_distributions().len(), 1);
        assert_eq!(simulation.state_transition_graph().node_count(), 2);
        assert_eq!(simulation.state_transition_graph().edge_count(), 0);
        assert_eq!(simulation.entropy(0), 1.0);
        dbg!(&simulation);

        simulation.next_step();

        assert_eq!(simulation.known_states().len(), 4);
        assert_eq!(simulation.known_transitions().len(), 2);
        assert_eq!(simulation.probability_distributions().len(), 2);
        assert_eq!(simulation.state_transition_graph().node_count(), 4);
        assert_eq!(simulation.state_transition_graph().edge_count(), 4);
        assert_eq!(simulation.entropy(1), 2.0);
        assert_eq!(
            simulation.probability_distributions(),
            HashMap::from([
                (0, HashMap::from([(0, 0.5), (1, 0.5)]),),
                (
                    1,
                    HashMap::from([(-1, 0.25), (0, 0.25), (1, 0.25), (2, 0.25)]),
                ),
            ])
        );

        dbg!(&simulation);
    }

    #[test]
    fn full_traversal() {
        let initial_state = 0;
        const NUM_STATES: i32 = 5;
        let state_transition_generator = Arc::new(|state: i32| -> OutgoingTransitions<i32, &str> {
            vec![
                (
                    {
                        if state + 1 == NUM_STATES {
                            0
                        } else {
                            state + 1
                        }
                    },
                    "forward",
                    0.5,
                ),
                (
                    {
                        if state - 1 == -1 {
                            NUM_STATES - 1
                        } else {
                            state - 1
                        }
                    },
                    "backward",
                    0.5,
                ),
            ]
        });
        let mut simulation = Simulation::new(initial_state, state_transition_generator);
        simulation.full_traversal(false);
        dbg!(&simulation);
        let graph = simulation.state_transition_graph();
        let dot = petgraph::dot::Dot::with_config(&graph, &[]);
        println!("{dot:#?}");
        assert_eq!(simulation.known_states().len(), NUM_STATES as usize);
        assert_eq!(simulation.known_transitions().len(), 2);
        assert_eq!(
            simulation.probability_distribution(simulation.time()).len(),
            4
        );
        assert_eq!(
            simulation.state_transition_graph().node_count(),
            NUM_STATES as usize
        );
        assert_eq!(
            simulation.state_transition_graph().edge_count(),
            2 * NUM_STATES as usize
        );

        let (transition_rate_matrix, ordering) = simulation.transition_rate_matrix();
        dbg!(&transition_rate_matrix);
        dbg!(&ordering);
        assert_eq!(transition_rate_matrix.nrows(), NUM_STATES as usize);
        assert_eq!(transition_rate_matrix.ncols(), NUM_STATES as usize);
        assert_eq!(
            transition_rate_matrix.sum_axis(Axis(0)),
            Array1::from_elem(NUM_STATES as usize, 1.0)
        );
        assert_eq!(
            transition_rate_matrix.sum_axis(Axis(1)),
            Array1::from_elem(NUM_STATES as usize, 1.0)
        );
        assert_eq!(transition_rate_matrix.get((0, 1)), Some(&0.5));
        assert_eq!(transition_rate_matrix.get((1, 0)), Some(&0.5));
    }

    #[test]
    fn uniform_distribution_is_steady() {
        {
            let initial_state = 0;
            const NUM_STATES: i32 = 5;
            let state_transition_generator =
                Arc::new(|state: i32| -> OutgoingTransitions<i32, &str> {
                    vec![
                        (
                            {
                                if state + 1 == NUM_STATES {
                                    0
                                } else {
                                    state + 1
                                }
                            },
                            "forward",
                            0.5,
                        ),
                        (
                            {
                                if state - 1 == -1 {
                                    NUM_STATES - 1
                                } else {
                                    state - 1
                                }
                            },
                            "backward",
                            0.5,
                        ),
                    ]
                });
            let mut simulation = Simulation::new(initial_state, state_transition_generator);
            assert!(simulation.uniform_distribution_is_steady());
        }
        {
            let initial_state = 0;
            const NUM_STATES: i32 = 5;
            let state_transition_generator =
                Arc::new(|state: i32| -> OutgoingTransitions<i32, &str> {
                    vec![
                        (
                            {
                                if state + 1 == NUM_STATES {
                                    1
                                } else {
                                    state + 1
                                }
                            },
                            "forward",
                            0.5,
                        ),
                        (0, "stay", 0.5),
                    ]
                });
            let mut simulation = Simulation::new(initial_state, state_transition_generator);
            assert!(!simulation.uniform_distribution_is_steady());
        }
    }
}
