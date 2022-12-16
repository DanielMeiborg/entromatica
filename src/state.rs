use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[allow(unused_imports)]
use hashbrown::{HashMap, HashSet};
#[allow(unused_imports)]
use itertools::Itertools;

use derive_more::*;
use rayon::prelude::*;

use crate::resources::*;
use crate::units::*;

use crate::rules::Action;

/// A single entity in the simulation.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Entity {
    pub resources: HashMap<ResourceName, Amount>,
}

impl Entity {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn get_resource(&self, resource_name: &ResourceName) -> Amount {
        *self
            .resources
            .get(resource_name)
            .expect("Resource {resource_name} not found")
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Display, Default, From, Into, AsRef, AsMut, Deref)]
pub struct EntityName(pub String);

impl EntityName {
    pub fn new() -> Self {
        Self("".to_string())
    }
}

/// A possible state in the markov chain of the simulation, which is only dependent on
/// the configuration of the entities in the simulation.
#[derive(Clone, Debug, Default)]
pub struct State {
    pub entities: HashMap<EntityName, Entity>,
}

impl Hash for State {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for (name, entity) in &self.entities {
            for (resource_name, amount) in &entity.resources {
                (name, resource_name, amount.to_bits()).hash(state);
            }
        }
    }
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        let self_hasher = &mut DefaultHasher::new();
        self.hash(self_hasher);
        let other_hasher = &mut DefaultHasher::new();
        other.hash(other_hasher);
        self_hasher.finish() == other_hasher.finish()
    }
}

impl Eq for State {}

impl State {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    pub fn get_entity(&self, entity_name: &EntityName) -> Entity {
        self.entities
            .get(entity_name)
            .expect("entity {entity_name} not found")
            .clone()
    }

    pub(crate) fn apply_actions(
        &self,
        actions: &Vec<Action>,
        resources: &HashMap<ResourceName, Resource>,
    ) -> State {
        let mut new_state = self.clone();
        for action in actions {
            new_state
                .entities
                .get_mut(&action.entity)
                .expect("Entity {action.entity} not found in state")
                .resources
                .insert(action.resource.clone(), action.new_amount);

            let capacity_per_entity = &resources
                .get(&action.resource)
                .expect("Resource {action.resource} not found in resources")
                .capacity_per_entity;

            if let Capacity::Limited(limit) = capacity_per_entity {
                if action.new_amount > *limit {
                    panic!(
                        "Resource limit per entity exceeded for resource {:#?}",
                        action.resource
                    );
                }
            }
        }
        new_state
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Display, Default)]
pub struct StateHash(pub u64);

impl StateHash {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn from_state(state: &State) -> Self {
        let mut hasher = &mut DefaultHasher::new();
        state.hash(&mut hasher);
        Self(hasher.finish())
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default, From, Into, AsRef, AsMut, Index, Deref)]
pub struct PossibleStates(pub HashMap<StateHash, State>);

impl PossibleStates {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub(crate) fn append_state(&mut self, state_hash: StateHash, state: State) {
        self.0.insert(state_hash, state);
    }

    pub(crate) fn append_states(&mut self, states: &PossibleStates) {
        for (state_hash, state) in states.0.iter() {
            self.append_state(*state_hash, state.clone());
        }
    }

    pub fn state(&self, state_hash: &StateHash) -> Option<State> {
        self.0.get(state_hash).cloned()
    }
}

#[derive(Clone, PartialEq, Debug, Default, From, Into, AsRef, AsMut, Index, Deref)]
pub struct ReachableStates(pub HashMap<StateHash, Probability>);

impl ReachableStates {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub(crate) fn append_state(&mut self, state_hash: StateHash, state_probability: Probability) {
        match self.0.get_mut(&state_hash) {
            Some(probability) => {
                *probability += state_probability;
            }
            None => {
                self.0.insert(state_hash, state_probability);
            }
        }
    }

    pub(crate) fn append_states(&mut self, states: &ReachableStates) {
        for (state_hash, state_probability) in states.iter() {
            self.append_state(*state_hash, *state_probability);
        }
    }

    pub fn values(&self) -> std::iter::Cloned<hashbrown::hash_map::Values<StateHash, Probability>> {
        self.0.values().cloned()
    }

    pub fn probability_sum(&self) -> Probability {
        Probability(self.par_iter().map(|(_, probability)| probability.0).sum())
    }

    /// Gets the entropy of the current probability distribution.
    pub fn entropy(&self) -> Entropy {
        Entropy(
            self.0
                .par_iter()
                .map(|(_, probability)| {
                    if *probability > Probability(0.) {
                        f64::from(*probability) * -f64::from(*probability).log2()
                    } else {
                        0.
                    }
                })
                .sum(),
        )
    }
}
