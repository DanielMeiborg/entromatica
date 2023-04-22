# Entromatica

![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/DanielMeiborg/entromatica/ci.yml?label=CI)
![Crates.io](https://img.shields.io/crates/l/entromatica)
![Crates.io](https://img.shields.io/crates/v/entromatica)
![GitHub release (latest SemVer including pre-releases)](https://img.shields.io/github/v/release/DanielMeiborg/entromatica?include_prereleases)
<a href="https://docs.rs/entromatica/"><img alt="API Docs" src="https://img.shields.io/badge/docs.rs-entromatica-orange"/></a>

**Entromatica is a library for constructing, simulating and analyzing markov
chains.**

It is split into two main parts: the `simulation` module and the `models` module
collection.

The `simulation` module contains primarily the `Simulation` struct, which takes
an initial state and a `StateTransitionGenerator`. This generator is a function
that takes a state and returns a list of the next states in the markov chain
with their respective relative probabilities.

The `models` module contains a collection of structs and functions that try
to make constructing the state transition generator easier. Currently this
includes only a single model: `rules`.

```rust
// This is a simple onedimensional random walk
use entromatica::prelude::*;
use std::sync::Arc;

// The initial state. It has to be Hash + Clone + Send + Sync + PartialEq + Eq + Debug
let initial_state: i32 = 0;

// The state transition generator. The simulation panics if the probabilities don't sum to 1.0
let state_transition_generator =
Arc::new(|state: i32| vec![(state + 1, "next", 0.5), (state - 1, "previous", 0.5)]);

let mut simulation = Simulation::new(initial_state, state_transition_generator);

// The Shannon-entropy at the given time
assert_eq!(simulation.entropy(0), 0.0);
simulation.next_step();
assert_eq!(simulation.entropy(1), 1.0);
```
## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
