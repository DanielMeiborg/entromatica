//! Entromatica is a library for constructing, simulating and analyzing markov
//! chains.
//!
//! It is split into two main parts: the [simulation](./simulation/index.html)
//! module and the [models](./models/index.html) module collection.
//!
//! The [simulation](./simulation/index.html) module contains primarily the
//! [Simulation](./simulation/struct.Simulation.html) struct, which takes an
//! initial state and a
//! [StateTransitionGenerator](./simulation/type.StateTransitionGenerator.html).
//! This generator is a function that takes a state and returns a list of the
//! next states in the markov chain with their respective relative
//! probabilities.
//!
//! The [models](./models/index.html) module contains a collection of of structs
//! and functions that try to make constructing the state transition generator
//! easier. As of v1.0.1 this includes only a single model: [rules](./models/rules/index.html).
//!
//! ```rust
//! // This is a simple onedimensional random walk
//! use entromatica::prelude::*;
//! use std::sync::Arc;
//!
//! // The initial state. It has to be Hash + Clone + Send + Sync + PartialEq + Eq + Debug
//! let initial_state: i32 = 0;
//!
//! // The state transition generator. The simulation panics if the probabilities don't sum to 1.0
//! let state_transition_generator =
//! Arc::new(|state: i32| vec![(state + 1, "next", 0.5), (state - 1, "previous", 0.5)]);
//!
//! let mut simulation = Simulation::new(initial_state, state_transition_generator);
//!
//! // The Shannon-entropy at the given time
//! assert_eq!(simulation.entropy(0), 0.0);
//! simulation.next_step();
//! assert_eq!(simulation.entropy(1), 1.0);
//! ```

mod cached_function;
mod hash;
pub mod models;
pub mod prelude;
pub mod simulation;
