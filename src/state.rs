use std::collections::hash_map::DefaultHasher;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::sync::mpsc::SendError;
use std::sync::Mutex;

#[allow(unused_imports)]
use hashbrown::{HashMap, HashSet};
#[allow(unused_imports)]
use itertools::Itertools;

use backtrace::Backtrace as trc;
use derive_more::*;
use rayon::prelude::*;
use thiserror::Error;

use crate::cache::*;
use crate::resource::*;
use crate::rules::*;
use crate::units::*;
use crate::*;

/// A single entity in the simulation.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Entity {
    resources: HashMap<ResourceName, Amount>,
}

impl Display for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Entity:")?;
        for (resource_name, amount) in &self.resources {
            writeln!(f, "  {resource_name}: {amount}")?;
        }
        Ok(())
    }
}

impl Entity {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }

    pub fn from_resources(resources: Vec<(ResourceName, Amount)>) -> Self {
        Self {
            resources: resources.into_iter().collect(),
        }
    }

    pub fn resource(&self, resource_name: &ResourceName) -> Result<&Amount, EntityError> {
        self.resources
            .get(resource_name)
            .ok_or_else(|| EntityError::ResourceNotFound {
                resource_name: resource_name.clone(),
                context: get_backtrace(),
            })
    }

    pub fn resource_mut(
        &mut self,
        resource_name: &ResourceName,
    ) -> Result<&mut Amount, EntityError> {
        self.resources
            .get_mut(resource_name)
            .ok_or_else(|| EntityError::ResourceNotFound {
                resource_name: resource_name.clone(),
                context: get_backtrace(),
            })
    }

    pub fn iter_resources(&self) -> impl Iterator<Item = (&ResourceName, &Amount)> {
        self.resources.iter()
    }

    pub fn iter_resources_mut(&mut self) -> impl Iterator<Item = (&ResourceName, &mut Amount)> {
        self.resources.iter_mut()
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Error)]
pub enum EntityError {
    #[error("Resource not found: {resource_name:#?}")]
    ResourceNotFound {
        resource_name: ResourceName,
        context: trc,
    },
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Display, Default, From, Into, AsRef, AsMut)]
pub struct EntityName(pub String);

impl EntityName {
    pub fn new() -> Self {
        Self("".to_string())
    }
}

/// A possible state in the markov chain of the simulation, which is only dependent on
/// the configuration of the entities in the simulation.
#[derive(Clone, Debug, Default, From, Into)]
pub struct State {
    entities: HashMap<EntityName, Entity>,
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "State:")?;
        for (entity_name, entity) in &self.entities {
            writeln!(f, "  {entity_name}:")?;
            for (resource_name, amount) in &entity.resources {
                writeln!(f, "    {resource_name}: {amount}")?;
            }
        }
        Ok(())
    }
}

impl Hash for State {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for (name, entity) in &self.entities {
            for (resource_name, amount) in &entity.resources {
                (name.clone(), resource_name.clone(), *amount).hash(state);
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

    pub fn from_entities(entities: Vec<(EntityName, Entity)>) -> Self {
        Self {
            entities: entities.into_iter().collect(),
        }
    }

    pub fn entity(&self, entity_name: &EntityName) -> Result<&Entity, StateError> {
        self.entities
            .get(entity_name)
            .ok_or_else(|| StateError::EntityNotFound {
                entity_name: entity_name.clone(),
                context: get_backtrace(),
            })
    }

    pub fn entity_mut(&mut self, entity_name: &EntityName) -> Result<&mut Entity, StateError> {
        self.entities
            .get_mut(entity_name)
            .ok_or_else(|| StateError::EntityNotFound {
                entity_name: entity_name.clone(),
                context: get_backtrace(),
            })
    }

    pub fn iter_entities(&self) -> impl Iterator<Item = (&EntityName, &Entity)> {
        self.entities.iter()
    }

    pub fn iter_entities_mut(&mut self) -> impl Iterator<Item = (&EntityName, &mut Entity)> {
        self.entities.iter_mut()
    }

    pub(crate) fn apply_actions(
        &self,
        actions: HashMap<ActionName, Action>,
    ) -> Result<State, StateError> {
        let mut new_state = self.clone();
        let mut affected_resources: HashSet<(EntityName, ResourceName)> = HashSet::new();
        for (_, action) in actions {
            if affected_resources.contains(&(action.target().clone(), action.resource().clone())) {
                return Err(StateError::ResourceAlreadyAffected {
                    resource_name: action.resource().clone(),
                    entity_name: action.target().clone(),
                    context: get_backtrace(),
                });
            } else {
                affected_resources.insert((action.target().clone(), action.resource().clone()));
            }
            new_state
                .entity_mut(action.target())?
                .resources
                .insert(action.resource().clone(), action.amount());
        }
        Ok(new_state)
    }

    pub(crate) fn reachable_states(
        &self,
        base_state_probability: &Probability,
        rules: &HashMap<RuleName, Rule>,
        possible_states: &PossibleStates,
        cache: &Cache,
        resources: &HashMap<ResourceName, Resource>,
    ) -> Result<
        (
            ReachableStates,
            PossibleStates,
            Vec<ConditionCacheUpdate>,
            Vec<ActionCacheUpdate>,
        ),
        ErrorKind,
    > {
        let base_state_hash = StateHash::from_state(self);
        let mut new_base_state_probability: Probability = *base_state_probability;
        let mut applying_rules_probability_weight_sum = ProbabilityWeight::from(0.);
        let mut reachable_states_by_rule_probability_weight: HashMap<StateHash, ProbabilityWeight> =
            HashMap::new();

        let mut condition_cache_updates = Vec::new();
        let mut action_cache_updates = Vec::new();

        let mut new_possible_states: PossibleStates = PossibleStates::new();

        for (rule_name, rule) in rules {
            let base_state = possible_states.state(&base_state_hash)?;
            let (rule_applies, condition_cache_update) =
                rule.applies(cache, rule_name.clone(), base_state.clone())?;
            if let Some(cache) = condition_cache_update {
                condition_cache_updates.push(cache);
            }
            if rule_applies.is_true() {
                new_base_state_probability *= 1. - f64::from(rule.weight());
                applying_rules_probability_weight_sum += rule.weight();
                let (new_state, action_cache_update) = rule.apply(
                    cache,
                    possible_states,
                    rule_name.clone(),
                    base_state_hash,
                    base_state.clone(),
                    resources,
                )?;
                if let Some(cache) = action_cache_update {
                    action_cache_updates.push(cache);
                }
                let new_state_hash = StateHash::from_state(&new_state);
                new_possible_states.append_state(new_state_hash, new_state)?;
                reachable_states_by_rule_probability_weight.insert(new_state_hash, rule.weight());
            }
        }

        let mut new_reachable_states = ReachableStates::new();

        if new_base_state_probability > Probability::from(0.) {
            new_reachable_states.append_state(base_state_hash, new_base_state_probability)?;
        }

        let probabilities_for_reachable_states_from_base_state = self
            .probabilities_for_reachable_states(
                reachable_states_by_rule_probability_weight,
                *base_state_probability,
                new_base_state_probability,
                applying_rules_probability_weight_sum,
            );

        for (new_state_hash, new_state_probability) in
            probabilities_for_reachable_states_from_base_state.iter()
        {
            new_reachable_states.append_state(*new_state_hash, *new_state_probability)?;
        }
        Ok((
            new_reachable_states,
            new_possible_states,
            condition_cache_updates,
            action_cache_updates,
        ))
    }

    fn probabilities_for_reachable_states(
        &self,
        reachable_states_by_rule_probability_weight: HashMap<StateHash, ProbabilityWeight>,
        base_state_probability: Probability,
        new_base_state_probability: Probability,
        applying_rules_probability_weight_sum: ProbabilityWeight,
    ) -> ReachableStates {
        ReachableStates::from(HashMap::from_par_iter(
            reachable_states_by_rule_probability_weight
                .par_iter()
                .filter_map(|(new_reachable_state_hash, rule_probability_weight)| {
                    if *new_reachable_state_hash != StateHash::from_state(self) {
                        let new_reachable_state_probability =
                            Probability::from_probability_weight(*rule_probability_weight)
                                * f64::from(base_state_probability)
                                * f64::from(Probability::from(1.) - new_base_state_probability)
                                / f64::from(applying_rules_probability_weight_sum);
                        Option::Some((*new_reachable_state_hash, new_reachable_state_probability))
                    } else {
                        Option::None
                    }
                }),
        ))
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Error)]
pub enum StateError {
    #[error("Entity not found: {entity_name:#?}")]
    EntityNotFound {
        entity_name: EntityName,
        context: trc,
    },

    #[error("Resource {resource_name:#?} already affected for entity {entity_name:#?}")]
    ResourceAlreadyAffected {
        resource_name: ResourceName,
        entity_name: EntityName,
        context: trc,
    },

    #[error("EntityError: {0:#?}")]
    EntityError(#[from] EntityError),
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Display, Default)]
pub struct StateHash(u64);

impl StateHash {
    pub fn new() -> Self {
        Self(Self::from_state(&State::new()).0)
    }

    pub fn from_state(state: &State) -> Self {
        let mut hasher = &mut DefaultHasher::new();
        state.hash(&mut hasher);
        Self(hasher.finish())
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default, From, Into, AsRef, AsMut, Index)]
pub struct PossibleStates(HashMap<StateHash, State>);

impl Display for PossibleStates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (state_hash, state) in &self.0 {
            writeln!(f, "{state_hash}: {state}")?;
        }
        Ok(())
    }
}

impl PossibleStates {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub(crate) fn append_state(
        &mut self,
        state_hash: StateHash,
        state: State,
    ) -> Result<(), PossibleStatesError> {
        match self.0.get(&state_hash) {
            Some(present_state) => {
                if state != *present_state {
                    Err(PossibleStatesError::StateAlreadyExists {
                        state_hash,
                        context: get_backtrace(),
                    })
                } else {
                    Ok(())
                }
            }
            None => {
                self.0.insert(state_hash, state);
                Ok(())
            }
        }
    }

    pub(crate) fn merge(&mut self, states: &PossibleStates) -> Result<(), ErrorKind> {
        for (state_hash, state) in states.iter() {
            self.append_state(*state_hash, state.clone())?;
        }
        Ok(())
    }

    pub fn state(&self, state_hash: &StateHash) -> Result<&State, PossibleStatesError> {
        self.0
            .get(state_hash)
            .ok_or_else(|| PossibleStatesError::StateNotFound {
                state_hash: *state_hash,
                context: get_backtrace(),
            })
    }

    pub fn iter(&self) -> hashbrown::hash_map::Iter<StateHash, State> {
        self.0.iter()
    }

    pub fn values(&self) -> hashbrown::hash_map::Values<StateHash, State> {
        self.0.values()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn contains(&self, state_hash: &StateHash) -> bool {
        self.0.contains_key(state_hash)
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Error)]
pub enum PossibleStatesError {
    #[error("State not found: {state_hash:#?}")]
    StateNotFound { state_hash: StateHash, context: trc },

    #[error("State already exists: {state_hash:#?}")]
    StateAlreadyExists { state_hash: StateHash, context: trc },

    #[error("Possible states send error: {source:#?}")]
    PossibleStatesSendError {
        #[source]
        source: SendError<PossibleStates>,
        context: trc,
    },
}

#[derive(Clone, PartialEq, Debug, Default, From, Into, AsRef, AsMut, Index)]
pub struct ReachableStates(HashMap<StateHash, Probability>);

impl Display for ReachableStates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (state_hash, probability) in &self.0 {
            writeln!(f, "{state_hash}: {probability}")?;
        }
        Ok(())
    }
}

impl ReachableStates {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn append_state(
        &mut self,
        state_hash: StateHash,
        state_probability: Probability,
    ) -> Result<(), UnitsError> {
        match self.0.get_mut(&state_hash) {
            Some(probability) => {
                if *probability + state_probability > Probability::from(1.) {
                    return Err(UnitsError::ProbabilityOutOfRange {
                        probability: *probability + state_probability,
                        context: get_backtrace(),
                    });
                }
                *probability += state_probability;
            }
            None => {
                self.0.insert(state_hash, state_probability);
            }
        }
        Ok(())
    }

    pub fn merge(&mut self, states: &ReachableStates) -> Result<(), ErrorKind> {
        for (state_hash, state_probability) in states.iter() {
            self.append_state(*state_hash, *state_probability)?;
        }
        Ok(())
    }

    pub fn values(&self) -> hashbrown::hash_map::Values<StateHash, Probability> {
        self.0.values()
    }

    pub fn iter(&self) -> hashbrown::hash_map::Iter<StateHash, Probability> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> hashbrown::hash_map::IterMut<StateHash, Probability> {
        self.0.iter_mut()
    }

    pub fn par_iter(
        &self,
    ) -> hashbrown::hash_map::rayon::ParIter<'_, state::StateHash, units::Probability> {
        self.0.par_iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn contains(&self, state_hash: &StateHash) -> bool {
        self.0.contains_key(state_hash)
    }

    pub fn probability_sum(&self) -> Probability {
        Probability::from(
            self.iter()
                .par_bridge()
                .map(|(_, probability)| probability.to_f64())
                .sum::<f64>(),
        )
    }

    /// Gets the entropy of the current probability distribution.
    pub fn entropy(&self) -> Entropy {
        Entropy::from(
            self.0
                .par_iter()
                .map(|(_, probability)| {
                    if *probability > Probability::from(0.) {
                        f64::from(*probability) * -f64::from(*probability).log2()
                    } else {
                        0.
                    }
                })
                .sum::<f64>(),
        )
    }

    /// Update reachable_states and possible_states to the next time step.
    pub(crate) fn apply_rules(
        &mut self,
        possible_states: &mut PossibleStates,
        cache: &mut Cache,
        resources: &HashMap<ResourceName, Resource>,
        rules: &HashMap<RuleName, Rule>,
    ) -> Result<(), ErrorKind> {
        let new_reachable_states_mutex = Mutex::new(ReachableStates::new());
        let possible_states_update_mutex = Mutex::new(PossibleStates::new());
        let cache_update_mutex = Mutex::new(cache.clone());

        self.par_iter()
            .map(|(base_state_hash, base_state_probability)| {
                possible_states.state(base_state_hash)?.reachable_states(
                    base_state_probability,
                    rules,
                    possible_states,
                    cache,
                    resources,
                )
            })
            .try_for_each(|result| {
                if let Ok((
                    new_reachable_states,
                    new_possible_states,
                    condition_cache_updates,
                    action_cache_updates,
                )) = result
                {
                    new_reachable_states_mutex
                        .lock()?
                        .merge(&new_reachable_states)?;
                    possible_states_update_mutex
                        .lock()?
                        .merge(&new_possible_states)?;
                    for condition_cache_update in condition_cache_updates {
                        cache_update_mutex
                            .lock()?
                            .apply_condition_update(condition_cache_update)?;
                    }
                    for action_cache_update in action_cache_updates {
                        cache_update_mutex
                            .lock()?
                            .apply_action_update(action_cache_update)?;
                    }
                    Ok(())
                } else {
                    Err(result.err().unwrap())
                }
            })?;

        if cfg!(debug_assertions) {
            let probability_sum = self.probability_sum();
            if probability_sum != Probability::from(1.) {
                return Err(ErrorKind::UnitsError(UnitsError::ProbabilitySumNot1 {
                    probability_sum,
                    context: get_backtrace(),
                }));
            }
        }

        *self = new_reachable_states_mutex.lock()?.clone();
        possible_states.merge(&possible_states_update_mutex.lock()?.clone())?;
        cache.merge(&cache_update_mutex.lock()?.clone())?;
        Ok(())
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Error)]
pub enum ReachableStatesError {
    #[error("State not found: {state_hash:#?}")]
    StateNotFound { state_hash: StateHash, context: trc },

    #[error("Reachable states send error: {source:#?}")]
    ReachableStatesSendError {
        #[source]
        source: SendError<ReachableStates>,
        context: trc,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_get_resource_should_return_value_on_present_resource() {
        let resources = vec![(ResourceName::from("resource".to_string()), Amount::from(1.))];
        let entity = Entity::from_resources(resources);
        assert_eq!(
            entity
                .resource(&ResourceName::from("resource".to_string()))
                .cloned()
                .unwrap(),
            Amount::from(1.)
        );
    }

    #[test]
    fn entity_get_resource_should_return_error_on_missing_resource() {
        let resources = vec![(ResourceName::from("resource".to_string()), Amount::from(1.))];
        let entity = Entity::from_resources(resources);
        if let Err(EntityError::ResourceNotFound { resource_name, .. }) =
            entity.resource(&ResourceName::from("missing_resource".to_string()))
        {
            assert_eq!(
                resource_name,
                ResourceName::from("missing_resource".to_string())
            );
        } else {
            panic!("Unexpected error type");
        }
    }

    #[test]
    fn state_partial_equal_works_as_expected() {
        let state_a_0 = State::from_entities(vec![(
            EntityName::from("A".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("Resource".to_string()),
                Amount::from(0.),
            )]),
        )]);
        let state_a_1 = State::from_entities(vec![(
            EntityName::from("A".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("Resource".to_string()),
                Amount::from(0.),
            )]),
        )]);
        let state_b = State::from_entities(vec![(
            EntityName::from("A".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("Resource".to_string()),
                Amount::from(1.),
            )]),
        )]);
        let state_c = State::from_entities(vec![(
            EntityName::from("B".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("Resource".to_string()),
                Amount::from(1.),
            )]),
        )]);
        assert_eq!(state_a_0, state_a_1);
        assert_ne!(state_a_0, state_b);
        assert_ne!(state_a_1, state_b);
        assert_ne!(state_b, state_c);
    }

    #[test]
    fn state_get_entity_should_return_value_on_present_entity() {
        let state = State::from_entities(vec![(
            EntityName::from("A".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("Resource".to_string()),
                Amount::from(0.),
            )]),
        )]);

        assert_eq!(
            state
                .entity(&EntityName::from("A".to_string()),)
                .cloned()
                .unwrap(),
            Entity::from_resources(vec![(
                ResourceName::from("Resource".to_string()),
                Amount::from(0.)
            )])
        );
    }

    #[test]
    fn state_get_entity_should_return_error_on_missing_entity() {
        let state = State::from_entities(vec![(
            EntityName::from("A".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("Resource".to_string()),
                Amount::from(0.),
            )]),
        )]);

        if let Err(StateError::EntityNotFound { entity_name, .. }) = state
            .entity(&EntityName::from("missing_entity".to_string()))
            .cloned()
        {
            assert_eq!(entity_name, EntityName::from("missing_entity".to_string()));
        } else {
            panic!("Unexpected error type");
        }
    }

    #[test]
    fn state_get_mut_entity_should_return_value_on_present_entity() {
        let mut state = State::from_entities(vec![(
            EntityName::from("A".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("Resource".to_string()),
                Amount::from(0.),
            )]),
        )]);

        assert_eq!(
            state
                .entity_mut(&EntityName::from("A".to_string()),)
                .unwrap(),
            &mut Entity::from_resources(vec![(
                ResourceName::from("Resource".to_string()),
                Amount::from(0.)
            )])
        );
    }

    #[test]
    fn state_get_mut_entity_should_return_error_on_missing_entity() {
        let mut state = State::from_entities(vec![(
            EntityName::from("A".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("Resource".to_string()),
                Amount::from(0.),
            )]),
        )]);

        if let Err(StateError::EntityNotFound { entity_name, .. }) = state
            .entity_mut(&EntityName::from("missing_entity".to_string()))
            .cloned()
        {
            assert_eq!(entity_name, EntityName::from("missing_entity".to_string()));
        } else {
            panic!("Unexpected error type");
        }
    }

    #[test]
    fn apply_actions_should_apply_actions_to_state() {
        let state = State::from_entities(vec![(
            EntityName::from("A".to_string()),
            Entity::from_resources(vec![
                (ResourceName::from("Resource".to_string()), Amount::from(0.)),
                (
                    ResourceName::from("Resource2".to_string()),
                    Amount::from(0.),
                ),
            ]),
        )]);
        let actions = HashMap::from([
            (
                ActionName::from("Action 1".to_string()),
                Action::from(
                    ResourceName::from("Resource".to_string()),
                    EntityName::from("A".to_string()),
                    Amount::from(1.),
                ),
            ),
            (
                ActionName::from("Action 2".to_string()),
                Action::from(
                    ResourceName::from("Resource2".to_string()),
                    EntityName::from("A".to_string()),
                    Amount::from(2.),
                ),
            ),
        ]);
        let new_state = state.apply_actions(actions).unwrap();
        assert_eq!(
            new_state,
            State::from_entities(vec![(
                EntityName::from("A".to_string()),
                Entity::from_resources(vec![
                    (ResourceName::from("Resource".to_string()), Amount::from(1.)),
                    (
                        ResourceName::from("Resource2".to_string()),
                        Amount::from(2.)
                    ),
                ]),
            )])
        );
    }

    #[test]
    fn apply_actions_should_return_error_on_multiple_actions_affecting_the_same_resource() {
        let state = State::from_entities(vec![(
            EntityName::from("A".to_string()),
            Entity::from_resources(vec![
                (ResourceName::from("Resource".to_string()), Amount::from(0.)),
                (
                    ResourceName::from("Resource2".to_string()),
                    Amount::from(0.),
                ),
            ]),
        )]);
        let actions = HashMap::from([
            (
                ActionName::from("Action 1".to_string()),
                Action::from(
                    ResourceName::from("Resource".to_string()),
                    EntityName::from("A".to_string()),
                    Amount::from(1.),
                ),
            ),
            (
                ActionName::from("Action 2".to_string()),
                Action::from(
                    ResourceName::from("Resource".to_string()),
                    EntityName::from("A".to_string()),
                    Amount::from(2.),
                ),
            ),
        ]);

        if let Err(StateError::ResourceAlreadyAffected {
            resource_name,
            entity_name,
            ..
        }) = state.apply_actions(actions)
        {
            assert_eq!(resource_name, ResourceName::from("Resource".to_string()));
            assert_eq!(entity_name, EntityName::from("A".to_string()));
        } else {
            panic!("Unexpected error type");
        }
    }

    #[test]
    fn possible_states_append_state() {
        let state = State::from_entities(vec![(
            EntityName::from("A".to_string()),
            Entity::from_resources(vec![
                (ResourceName::from("Resource".to_string()), Amount::from(0.)),
                (
                    ResourceName::from("Resource2".to_string()),
                    Amount::from(0.),
                ),
            ]),
        )]);
        let state_hash = StateHash::from_state(&state);
        let mut possible_states = PossibleStates::new();
        possible_states
            .append_state(state_hash, state.clone())
            .unwrap();
        let expected = HashMap::from([(state_hash, state)]);
        assert_eq!(possible_states.0, expected);

        let new_state = State::from_entities(vec![(
            EntityName::from("A".to_string()),
            Entity::from_resources(vec![
                (ResourceName::from("Resource".to_string()), Amount::from(1.)),
                (
                    ResourceName::from("Resource2".to_string()),
                    Amount::from(2.),
                ),
            ]),
        )]);

        possible_states
            .append_state(state_hash, new_state)
            .unwrap_err();
        assert_eq!(possible_states.0, expected);
    }

    #[test]
    fn reachable_states_append_state() {
        let mut reachable_states = ReachableStates::new();
        let state_hash = StateHash::new();
        let probability = Probability::from(1.);
        reachable_states
            .append_state(state_hash, probability)
            .unwrap();
        let expected = HashMap::from([(state_hash, probability)]);
        assert_eq!(reachable_states.0, expected);

        reachable_states
            .append_state(state_hash, probability)
            .unwrap_err();
        assert_eq!(reachable_states.0, expected);
    }

    #[test]
    fn reachable_states_probability_sum() {
        let mut reachable_states = ReachableStates::new();
        let state_hash = StateHash::new();
        let probability = Probability::from(0.2);
        reachable_states
            .append_state(state_hash, probability)
            .unwrap();
        let state_hash = StateHash::from_state(&State::from_entities(vec![(
            EntityName::from("A".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("Resource".to_string()),
                Amount::from(0.),
            )]),
        )]));
        let probability = Probability::from(0.5);
        reachable_states
            .append_state(state_hash, probability)
            .unwrap();
        assert_eq!(reachable_states.probability_sum(), Probability::from(0.7));
    }

    #[test]
    fn reachable_states_entropy() {
        let mut reachable_states = ReachableStates::new();
        assert_eq!(reachable_states.entropy(), Entropy::from(0.));
        let state_hash = StateHash::new();
        let probability = Probability::from(0.5);
        reachable_states
            .append_state(state_hash, probability)
            .unwrap();
        let state_hash = StateHash::from_state(&State::from_entities(vec![(
            EntityName::from("A".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("Resource".to_string()),
                Amount::from(0.),
            )]),
        )]));
        let probability = Probability::from(0.5);
        reachable_states
            .append_state(state_hash, probability)
            .unwrap();
        assert_eq!(reachable_states.entropy(), Entropy::from(1.));
    }
}
