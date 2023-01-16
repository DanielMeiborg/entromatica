# Changelog

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
