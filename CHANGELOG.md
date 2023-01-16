# Changelog

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
