use std::{
    collections::hash_map::DefaultHasher,
    fmt::Debug,
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};

use crate::prelude::*;
use hashbrown::HashMap;
use petgraph::{graph::Graph, visit::EdgeRef};
use rayon::prelude::*;

type StateHash = u64;
type KnownStates<S> = HashMap<StateHash, S>;

type TransitionHash = u64;
type KnownTransitions<T> = HashMap<TransitionHash, T>;

type StateTransitionGraph = Graph<StateHash, (TransitionHash, Probability)>;

type StateTransitionGenerator<S, T> = CachedFunction<S, StateTransitionProbabilities<S, T>>;

type StateProbabilityDistribution<S> = HashMap<S, Probability>;

type StateTransitionProbabilities<S, T> = HashMap<S, (T, Probability)>;

type HashedStateProbabilityDistribution = HashMap<StateHash, Probability>;

type Probability = f64;
type Time = u64;

fn hash(hashable: &impl Hash) -> StateHash {
    let mut hasher = DefaultHasher::new();
    hashable.hash(&mut hasher);
    hasher.finish()
}

#[derive(Clone)]
pub struct Simulation<S, T> {
    state_transition_graph: StateTransitionGraph,
    probability_distributions: HashMap<Time, HashedStateProbabilityDistribution>,
    known_states: KnownStates<S>,
    known_transitions: KnownTransitions<T>,
    state_transition_generator: StateTransitionGenerator<S, T>,
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
    pub fn new(
        initial_state: S,
        state_transition_generator: Arc<
            dyn Fn(S) -> StateTransitionProbabilities<S, T> + Send + Sync + 'static,
        >,
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

    pub fn new_with_distribution(
        probabilities: StateProbabilityDistribution<S>,
        state_transition_generator: Arc<
            dyn Fn(S) -> StateTransitionProbabilities<S, T> + Send + Sync + 'static,
        >,
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

    pub fn state_transition_graph(&self) -> Graph<S, (T, Probability)> {
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
        let mut graph = Graph::new();
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

    pub fn state_probability(&self, state: S, time: Time) -> f64 {
        self.probability_distributions
            .get(&time)
            .and_then(|state_probability_distribution| {
                state_probability_distribution.get(&hash(&state))
            })
            .copied()
            .unwrap_or(0.0)
    }

    pub fn initial_distribution(&self) -> StateProbabilityDistribution<S> {
        self.probability_distribution(0)
    }

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

    pub fn known_states(&self) -> Vec<S> {
        self.known_states.values().cloned().collect()
    }

    pub fn known_transitions(&self) -> Vec<T> {
        self.known_transitions.values().cloned().collect()
    }

    pub fn entropy(&self, time: Time) -> f64 {
        let state_probability_distribution = self.probability_distribution(time);
        let entropy = state_probability_distribution
            .values()
            .map(|probability| probability * probability.log2())
            .sum::<f64>()
            .abs();
        entropy
    }

    pub fn time(&self) -> Time {
        self.probability_distributions
            .keys()
            .max()
            .copied()
            .unwrap_or(0)
    }

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

        state_transition_probabilities
            .par_iter()
            .for_each(|next_states| {
                assert_eq!(
                    next_states
                        .iter()
                        .map(|(_, (_, probability))| probability)
                        .sum::<Probability>(),
                    1.0,
                    "Sum of probabilities of next states is not 1.0"
                );
            });

        let new_hashed_state_probability_distribution_mutex = Mutex::new(HashMap::new());
        state_transition_probabilities
            .par_iter()
            .zip_eq(state_probability_distribution.par_iter())
            .for_each(|(next_states, (_, current_state_probability))| {
                next_states
                    .iter()
                    .for_each(|(new_state, (_, probability))| {
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
        self.probability_distributions.insert(
            initial_time + 1,
            new_hashed_state_probability_distribution_mutex
                .into_inner()
                .unwrap(),
        );

        state_transition_probabilities
            .iter()
            .for_each(|next_states| {
                next_states.iter().for_each(|(new_state, (transition, _))| {
                    self.known_states.insert(hash(new_state), new_state.clone());
                    self.known_transitions
                        .insert(hash(transition), transition.clone());
                });
            });

        state_transition_probabilities
            .iter()
            .zip(state_probability_distribution.iter())
            .for_each(|(next_states, (old_state, _))| {
                next_states
                    .iter()
                    .for_each(|(new_state, (transition, probability))| {
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

        self.probability_distribution(initial_time + 1)
    }

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_walk() {
        let initial_state = 0;
        let state_transition_generator = Arc::new(|state: i32| {
            HashMap::from([(state + 1, ("next", 0.5)), (state - 1, ("previous", 0.5))])
        });

        let mut simulation = Simulation::new(initial_state, state_transition_generator);
        assert_eq!(simulation.known_states.len(), 1);
        assert_eq!(simulation.known_transitions.len(), 0);
        assert_eq!(simulation.probability_distributions.len(), 1);
        assert_eq!(simulation.state_transition_graph.node_count(), 1);
        assert_eq!(simulation.state_transition_graph.edge_count(), 0);
        assert_eq!(simulation.entropy(0), 0.0);
        dbg!(&simulation);

        simulation.next_step();
        assert_eq!(simulation.known_states.len(), 3);
        assert_eq!(simulation.known_transitions.len(), 2);
        assert_eq!(simulation.probability_distributions.len(), 2);
        assert_eq!(simulation.state_transition_graph.node_count(), 3);
        assert_eq!(simulation.state_transition_graph.edge_count(), 2);
        assert_eq!(simulation.entropy(1), 1.0);
        dbg!(&simulation);

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
        let state_transition_generator = Arc::new(|state: i32| -> HashMap<i32, (&str, f64)> {
            HashMap::from([(state + 1, ("next", 0.5)), (state - 1, ("previous", 0.5))])
        });
        let mut simulation =
            Simulation::new_with_distribution(initial_distribution, state_transition_generator);
        assert_eq!(simulation.known_states.len(), 2);
        assert_eq!(simulation.known_transitions.len(), 0);
        assert_eq!(simulation.probability_distributions.len(), 1);
        assert_eq!(simulation.state_transition_graph.node_count(), 2);
        assert_eq!(simulation.state_transition_graph.edge_count(), 0);
        assert_eq!(simulation.entropy(0), 1.0);
        dbg!(&simulation);

        simulation.next_step();

        assert_eq!(simulation.known_states.len(), 4);
        assert_eq!(simulation.known_transitions.len(), 2);
        assert_eq!(simulation.probability_distributions.len(), 2);
        assert_eq!(simulation.state_transition_graph.node_count(), 4);
        assert_eq!(simulation.state_transition_graph.edge_count(), 4);
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
        let state_transition_generator = Arc::new(|state: i32| -> HashMap<i32, (&str, f64)> {
            HashMap::from([
                (
                    {
                        if state + 1 == NUM_STATES {
                            0
                        } else {
                            state + 1
                        }
                    },
                    ("forward", 0.5),
                ),
                (
                    {
                        if state - 1 == -1 {
                            NUM_STATES - 1
                        } else {
                            state - 1
                        }
                    },
                    ("backward", 0.5),
                ),
            ])
        });
        let mut simulation = Simulation::new(initial_state, state_transition_generator);
        simulation.full_traversal(false);
        dbg!(&simulation);
        let graph = simulation.state_transition_graph();
        let dot = petgraph::dot::Dot::with_config(&graph, &[]);
        println!("{dot:#?}");
        assert_eq!(simulation.known_states.len(), NUM_STATES as usize);
        assert_eq!(simulation.known_transitions.len(), 2);
        assert_eq!(
            simulation.probability_distribution(simulation.time()).len(),
            4
        );
        assert_eq!(
            simulation.state_transition_graph.node_count(),
            NUM_STATES as usize
        );
        assert_eq!(
            simulation.state_transition_graph.edge_count(),
            2 * NUM_STATES as usize
        );
    }

    #[test]
    fn uniform_distribution_is_steady() {
        {
            let initial_state = 0;
            const NUM_STATES: i32 = 5;
            let state_transition_generator = Arc::new(|state: i32| -> HashMap<i32, (&str, f64)> {
                HashMap::from([
                    (
                        {
                            if state + 1 == NUM_STATES {
                                0
                            } else {
                                state + 1
                            }
                        },
                        ("forward", 0.5),
                    ),
                    (
                        {
                            if state - 1 == -1 {
                                NUM_STATES - 1
                            } else {
                                state - 1
                            }
                        },
                        ("backward", 0.5),
                    ),
                ])
            });
            let mut simulation = Simulation::new(initial_state, state_transition_generator);
            assert!(simulation.uniform_distribution_is_steady());
        }
        {
            let initial_state = 0;
            const NUM_STATES: i32 = 5;
            let state_transition_generator = Arc::new(|state: i32| -> HashMap<i32, (&str, f64)> {
                HashMap::from([
                    (
                        {
                            if state + 1 == NUM_STATES {
                                1
                            } else {
                                state + 1
                            }
                        },
                        ("forward", 0.5),
                    ),
                    (0, ("stay", 0.5)),
                ])
            });
            let mut simulation = Simulation::new(initial_state, state_transition_generator);
            assert!(!simulation.uniform_distribution_is_steady());
        }
    }
}
