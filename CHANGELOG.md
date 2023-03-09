# Changelog

## [1.0.1](https://github.com/Entromatica/entromatica/compare/v1.0.0...v1.0.1) (2023-03-09)


### Bug Fixes

* panics on debug mode with attempt to multiply with overflow ([6ed567b](https://github.com/Entromatica/entromatica/commit/6ed567bea77f6926c0c0fa52f6cd87b33191e494))

## [1.0.0](https://github.com/Entromatica/entromatica/compare/v0.23.0...v1.0.0) (2023-03-09)


### ⚠ BREAKING CHANGES

* About everything

### Features

* Rule mechanism for seperated frontend/backend ([12f38ad](https://github.com/Entromatica/entromatica/commit/12f38ad16943b9d127fce0cde41294a36c586e75))
* **separation:** generic simulation ([e39f0d0](https://github.com/Entromatica/entromatica/commit/e39f0d0715025f551e2550edc1ed2801bbf6bf8e))


### Reverts

* revert ci(release-please): Add write permissions ([c12a47e](https://github.com/Entromatica/entromatica/commit/c12a47e7e1c670fefceecba46203ac7d2fef165a))

## [0.23.0](https://github.com/DanielMeiborg/entromatica/compare/v0.22.1...v0.23.0) (2023-01-28)


### ⚠ BREAKING CHANGES

* Rule now uses closures
* Entities now have to be constructed with generic parameters. Amount has been conceptually replaced with Parameter.

### Features

* closures for rules ([cdfcf7a](https://github.com/DanielMeiborg/entromatica/commit/cdfcf7af08618679088f3289c9e6c4a186dc621c))
* Generic parameters for Entity ([c44821f](https://github.com/DanielMeiborg/entromatica/commit/c44821ff85ed7a37e82ec9f628c465824fae2e3a))


### Bug Fixes

* clippy warning 'variables can be used directly in the `format!` string' ([64f3f32](https://github.com/DanielMeiborg/entromatica/commit/64f3f329d201bbf4c8e8247b761d8501e2a474dc))

## [0.22.1](https://github.com/DanielMeiborg/entromatica/compare/v0.22.0...v0.22.1) (2023-01-23)


### Features

* add serde support to everything but rules and simulation ([9dc0d36](https://github.com/DanielMeiborg/entromatica/commit/9dc0d36b74aaad8ff5b7b13dc766c49450e459aa))
* SerializableSimulation, closes [#38](https://github.com/DanielMeiborg/entromatica/issues/38) ([c70c765](https://github.com/DanielMeiborg/entromatica/commit/c70c7652596663e315b6cda2404547d3a8d09f01))

## [0.22.0](https://github.com/DanielMeiborg/entromatica/compare/v0.21.1...v0.22.0) (2023-01-22)


### ⚠ BREAKING CHANGES

* new() constructors for units now take a value instead of using 0.

### Features

* Add euclidean_norm to ReachableStates, closes [#30](https://github.com/DanielMeiborg/entromatica/issues/30) ([dcbb2ce](https://github.com/DanielMeiborg/entromatica/commit/dcbb2ce5f2c03e4a9ac4e6c69669d9cd7dbc8ee4))
* new() constructors for units now take a value instead of using 0. ([a67e463](https://github.com/DanielMeiborg/entromatica/commit/a67e4635b9136b8588a3dd901955b63d9a4efaeb))
* ReachableStates::probability ([2f44d0b](https://github.com/DanielMeiborg/entromatica/commit/2f44d0b832e1e82e1f90adf88e35c2fb742976ce))
* Simulation::clone_without_history ([fafa3ed](https://github.com/DanielMeiborg/entromatica/commit/fafa3ed471314906eed737f37886480f89099f5e))

## [0.21.1](https://github.com/DanielMeiborg/entromatica/compare/v0.21.0...v0.21.1) (2023-01-16)


### Features

* Implement Iterator for Simulation, closes [#66](https://github.com/DanielMeiborg/entromatica/issues/66) ([c298d34](https://github.com/DanielMeiborg/entromatica/commit/c298d340458edfc40085b40662ce9fe7795c6984))

## [0.21.0](https://github.com/DanielMeiborg/entromatica/compare/v0.20.0...v0.21.0) (2023-01-16)


### ⚠ BREAKING CHANGES

* history for simulation, reachable states as initial_state, replaced Time with usize, closes #21, #57

### Features

* history for simulation, reachable states as initial_state, replaced Time with usize, closes [#21](https://github.com/DanielMeiborg/entromatica/issues/21), [#57](https://github.com/DanielMeiborg/entromatica/issues/57) ([fcd3dfd](https://github.com/DanielMeiborg/entromatica/commit/fcd3dfd04947554d48471eb0924dad10aab2cec5))

## [0.20.0](https://github.com/DanielMeiborg/entromatica/compare/v0.19.0...v0.20.0) (2023-01-16)


### ⚠ BREAKING CHANGES

* Proper error handling for graph()

### Features

* Proper error handling for graph() ([5c0e3ce](https://github.com/DanielMeiborg/entromatica/commit/5c0e3ce848b4a6425f6053bd6984dad46dc93164))

## [0.19.0](https://github.com/DanielMeiborg/entromatica/compare/v0.18.1...v0.19.0) (2023-01-16)


### ⚠ BREAKING CHANGES

* uniform_distribution_is_steady has an iteration_limit
* Add modify_state option to Simulation::full_traversal

### Features

* Add modify_state option to Simulation::full_traversal ([3e1beb7](https://github.com/DanielMeiborg/entromatica/commit/3e1beb72aeac137f018fffe34f92a853b5a681d2))
* uniform_distribution_is_steady has an iteration_limit ([0121091](https://github.com/DanielMeiborg/entromatica/commit/0121091f90fa0c9d02f62f5c1e193b6bb3d5af1a))

## [0.18.1](https://github.com/DanielMeiborg/entromatica/compare/v0.18.0...v0.18.1) (2023-01-16)


### Bug Fixes

* Condition::{Never, Always} does not create cache updates ([014f075](https://github.com/DanielMeiborg/entromatica/commit/014f075ecf98b26125f2322f734ec069df598216))

## [0.18.0](https://github.com/DanielMeiborg/entromatica/compare/v0.17.2...v0.18.0) (2023-01-16)


### ⚠ BREAKING CHANGES

* Refactor rules
* from(...) main constructors are now named new(...)
* `resource`is now called `parameter` and capacities are removed

### Features

* Convert from(...) to new(...) methods ([2d464e7](https://github.com/DanielMeiborg/entromatica/commit/2d464e7dc39a7a06f87aa4e56187e8a2b0bbc759))
* Convert resources to parameters without capacity checks ([3842374](https://github.com/DanielMeiborg/entromatica/commit/3842374545e373948847171aa615f91d0c98b3ba))
* full traversal method to run the simulation until the whole graph has been explored ([688c8f6](https://github.com/DanielMeiborg/entromatica/commit/688c8f668a472fb5762b1bbce98fd90d9b0f9dcc)), closes [#65](https://github.com/DanielMeiborg/entromatica/issues/65)
* Refactor rules ([1188d32](https://github.com/DanielMeiborg/entromatica/commit/1188d32fbb5d34427b528a17c9b70e5fb8b1ea44))


### Bug Fixes

* Simulation::apply_intervention breaks when rule points state to to itself ([0b5a574](https://github.com/DanielMeiborg/entromatica/commit/0b5a5740ede09d8cbc15950451857ea520aee69f))

## [0.17.2](https://github.com/DanielMeiborg/entromatica/compare/v0.17.1...v0.17.2) (2023-01-12)


### Features

* conversion from u64 to StateHash ([e439bd7](https://github.com/DanielMeiborg/entromatica/commit/e439bd7c1f15f56e0995ac71426db395ad152388))

## [0.17.1](https://github.com/DanielMeiborg/entromatica/compare/v0.17.0...v0.17.1) (2023-01-11)


### Bug Fixes

* **changelog:** release-please workflow does not get current branch ([b9b4385](https://github.com/DanielMeiborg/entromatica/commit/b9b438535a490cfad487fda6383d2a2613aa2404))
* **changelog:** Use release-please only for main branch ([724364c](https://github.com/DanielMeiborg/entromatica/commit/724364c609d32328ca113314c1038eabf9ee103e))

## Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org).
