use std::collections::hash_map::DefaultHasher;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::sync::mpsc::SendError;
use std::sync::Mutex;

use backtrace::Backtrace as trc;
use derive_more::*;
use hashbrown::HashMap;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::prelude::*;

#[derive(
    Clone,
    PartialEq,
    Eq,
    Hash,
    Debug,
    Display,
    Default,
    From,
    AsRef,
    AsMut,
    Into,
    Serialize,
    Deserialize,
)]
pub struct ParameterName(String);

impl ParameterName {
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }
}

#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Entity {
    parameters: HashMap<ParameterName, Amount>,
}

impl Display for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Entity:")?;
        for (parameter_name, amount) in &self.parameters {
            writeln!(f, "  {parameter_name}: {amount}")?;
        }
        Ok(())
    }
}

impl Entity {
    pub fn new(parameters: Vec<(ParameterName, Amount)>) -> Self {
        Self {
            parameters: parameters.into_iter().collect(),
        }
    }

    pub fn parameter(&self, parameter_name: &ParameterName) -> Result<&Amount, EntityError> {
        self.parameters
            .get(parameter_name)
            .ok_or_else(|| EntityError::ParameterNotFound {
                parameter_name: parameter_name.clone(),
                context: get_backtrace(),
            })
    }

    pub fn parameter_mut(
        &mut self,
        parameter_name: &ParameterName,
    ) -> Result<&mut Amount, EntityError> {
        self.parameters
            .get_mut(parameter_name)
            .ok_or_else(|| EntityError::ParameterNotFound {
                parameter_name: parameter_name.clone(),
                context: get_backtrace(),
            })
    }

    pub fn iter_parameters(&self) -> impl Iterator<Item = (&ParameterName, &Amount)> {
        self.parameters.iter()
    }

    pub fn iter_parameters_mut(&mut self) -> impl Iterator<Item = (&ParameterName, &mut Amount)> {
        self.parameters.iter_mut()
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Error)]
pub enum EntityError {
    #[error("Parameter not found: {parameter_name:#?}")]
    ParameterNotFound {
        parameter_name: ParameterName,
        context: trc,
    },
}

#[derive(
    Clone,
    PartialEq,
    Eq,
    Hash,
    Debug,
    Display,
    Default,
    From,
    Into,
    AsRef,
    AsMut,
    Serialize,
    Deserialize,
)]
pub struct EntityName(pub String);

impl EntityName {
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }
}

#[derive(Clone, Debug, Default, From, Into, Serialize, Deserialize)]
pub struct State {
    entities: HashMap<EntityName, Entity>,
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "State:")?;
        for (entity_name, entity) in &self.entities {
            writeln!(f, "  {entity_name}:")?;
            for (parameter_name, amount) in &entity.parameters {
                writeln!(f, "    {parameter_name}: {amount}")?;
            }
        }
        Ok(())
    }
}

impl Hash for State {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for (name, entity) in &self.entities {
            for (parameter_name, amount) in &entity.parameters {
                (name.clone(), parameter_name.clone(), *amount).hash(state);
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
    pub fn new(entities: Vec<(EntityName, Entity)>) -> Self {
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

    pub fn insert_entity(&mut self, entity_name: EntityName, entity: Entity) {
        self.entities.insert(entity_name, entity);
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

    pub(crate) fn adjust_parameter(
        &mut self,
        target: &EntityName,
        parameter: ParameterName,
        amount: Amount,
    ) -> Result<(), StateError> {
        let entity = self.entity_mut(target)?;
        let parameter = entity.parameter_mut(&parameter)?;
        *parameter += amount;
        Ok(())
    }

    pub(crate) fn set_parameter(
        &mut self,
        target: &EntityName,
        parameter: ParameterName,
        amount: Amount,
    ) -> Result<(), StateError> {
        let entity = self.entity_mut(target)?;
        let parameter = entity.parameter_mut(&parameter)?;
        *parameter = amount;
        Ok(())
    }

    pub(crate) fn reachable_states(
        &self,
        base_state_probability: &Probability,
        rules: &HashMap<RuleName, Rule>,
        possible_states: &PossibleStates,
        cache: &Cache,
    ) -> Result<
        (
            ReachableStates,
            PossibleStates,
            Vec<ConditionCacheUpdate>,
            Vec<ActionCacheUpdate>,
        ),
        ErrorKind,
    > {
        let base_state_hash = StateHash::new(self);
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
                applying_rules_probability_weight_sum += rule.weight();
                let (new_state, action_cache_update) = rule.apply(
                    cache,
                    possible_states,
                    rule_name.clone(),
                    base_state_hash,
                    base_state.clone(),
                )?;
                if &new_state != self {
                    new_base_state_probability *= 1. - f64::from(rule.weight());
                }
                if let Some(cache) = action_cache_update {
                    action_cache_updates.push(cache);
                }
                let new_state_hash = StateHash::new(&new_state);
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
                    if *new_reachable_state_hash != StateHash::new(self) {
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

    #[error("Parameter {parameter_name:#?} already affected for entity {entity_name:#?}")]
    ParameterAlreadyAffected {
        parameter_name: ParameterName,
        entity_name: EntityName,
        context: trc,
    },

    #[error("EntityError: {0:#?}")]
    EntityError(#[from] EntityError),
}

#[derive(
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Debug,
    Display,
    Default,
    From,
    Into,
    AsRef,
    AsMut,
    Serialize,
    Deserialize,
)]
pub struct StateHash(u64);

impl StateHash {
    pub fn new(state: &State) -> Self {
        let mut hasher = &mut DefaultHasher::new();
        state.hash(&mut hasher);
        Self(hasher.finish())
    }
}

#[derive(
    Clone, PartialEq, Eq, Debug, Default, From, Into, AsRef, AsMut, Index, Serialize, Deserialize,
)]
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

#[derive(
    Clone, PartialEq, Debug, Default, From, Into, AsRef, AsMut, Index, Serialize, Deserialize,
)]
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

    pub fn probability(&self, state_hash: &StateHash) -> Probability {
        if let Some(probability) = self.0.get(state_hash) {
            *probability
        } else {
            Probability::from(0.)
        }
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
                if state_probability > Probability::from(0.) {
                    self.0.insert(state_hash, state_probability);
                }
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

    pub fn par_iter(&self) -> hashbrown::hash_map::rayon::ParIter<'_, StateHash, Probability> {
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

    pub fn entropy(&self) -> Entropy {
        Entropy::from(
            self.par_iter()
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

    pub fn euclidean_norm(&self, base: &ReachableStates) -> Entropy {
        Entropy::from(
            self.par_iter()
                .map(|(state_hash, probability)| {
                    let base_state_probability = base.probability(state_hash);
                    (probability.to_f64() - base_state_probability.to_f64()).powi(2)
                })
                .sum::<f64>()
                .sqrt(),
        )
    }

    pub(crate) fn apply_rules(
        &self,
        possible_states: &mut PossibleStates,
        cache: &mut Cache,
        rules: &HashMap<RuleName, Rule>,
    ) -> Result<ReachableStates, ErrorKind> {
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

        possible_states.merge(&possible_states_update_mutex.lock()?.clone())?;
        cache.merge(&cache_update_mutex.lock()?.clone())?;
        let new_reachable_states = new_reachable_states_mutex.lock()?.clone();
        Ok(new_reachable_states)
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
    fn entity_get_parameter_should_return_value_on_present_parameter() {
        let parameters = vec![(ParameterName::new("parameter"), Amount::from(1.))];
        let entity = Entity::new(parameters);
        assert_eq!(
            entity
                .parameter(&ParameterName::new("parameter"))
                .cloned()
                .unwrap(),
            Amount::from(1.)
        );
    }

    #[test]
    fn entity_get_parameter_should_return_error_on_missing_parameter() {
        let parameters = vec![(ParameterName::new("parameter"), Amount::from(1.))];
        let entity = Entity::new(parameters);
        if let Err(EntityError::ParameterNotFound { parameter_name, .. }) =
            entity.parameter(&ParameterName::new("missing_parameter"))
        {
            assert_eq!(parameter_name, ParameterName::new("missing_parameter"));
        } else {
            panic!("Unexpected error type");
        }
    }

    #[test]
    fn state_partial_equal_works_as_expected() {
        let state_a_0 = State::new(vec![(
            EntityName::new("A"),
            Entity::new(vec![(ParameterName::new("Parameter"), Amount::from(0.))]),
        )]);
        let state_a_1 = State::new(vec![(
            EntityName::new("A"),
            Entity::new(vec![(ParameterName::new("Parameter"), Amount::from(0.))]),
        )]);
        let state_b = State::new(vec![(
            EntityName::new("A"),
            Entity::new(vec![(ParameterName::new("Parameter"), Amount::from(1.))]),
        )]);
        let state_c = State::new(vec![(
            EntityName::new("B"),
            Entity::new(vec![(ParameterName::new("Parameter"), Amount::from(1.))]),
        )]);
        assert_eq!(state_a_0, state_a_1);
        assert_ne!(state_a_0, state_b);
        assert_ne!(state_a_1, state_b);
        assert_ne!(state_b, state_c);
    }

    #[test]
    fn state_get_entity_should_return_value_on_present_entity() {
        let state = State::new(vec![(
            EntityName::new("A"),
            Entity::new(vec![(ParameterName::new("Parameter"), Amount::from(0.))]),
        )]);

        assert_eq!(
            state.entity(&EntityName::new("A"),).cloned().unwrap(),
            Entity::new(vec![(ParameterName::new("Parameter"), Amount::from(0.))])
        );
    }

    #[test]
    fn state_get_entity_should_return_error_on_missing_entity() {
        let state = State::new(vec![(
            EntityName::new("A"),
            Entity::new(vec![(ParameterName::new("Parameter"), Amount::from(0.))]),
        )]);

        if let Err(StateError::EntityNotFound { entity_name, .. }) =
            state.entity(&EntityName::new("missing_entity")).cloned()
        {
            assert_eq!(entity_name, EntityName::new("missing_entity"));
        } else {
            panic!("Unexpected error type");
        }
    }

    #[test]
    fn state_get_mut_entity_should_return_value_on_present_entity() {
        let mut state = State::new(vec![(
            EntityName::new("A"),
            Entity::new(vec![(ParameterName::new("Parameter"), Amount::from(0.))]),
        )]);

        assert_eq!(
            state.entity_mut(&EntityName::new("A"),).unwrap(),
            &mut Entity::new(vec![(ParameterName::new("Parameter"), Amount::from(0.))])
        );
    }

    #[test]
    fn state_get_mut_entity_should_return_error_on_missing_entity() {
        let mut state = State::new(vec![(
            EntityName::new("A"),
            Entity::new(vec![(ParameterName::new("Parameter"), Amount::from(0.))]),
        )]);

        if let Err(StateError::EntityNotFound { entity_name, .. }) = state
            .entity_mut(&EntityName::new("missing_entity"))
            .cloned()
        {
            assert_eq!(entity_name, EntityName::new("missing_entity"));
        } else {
            panic!("Unexpected error type");
        }
    }

    #[test]
    fn possible_states_append_state() {
        let state = State::new(vec![(
            EntityName::new("A"),
            Entity::new(vec![
                (ParameterName::new("Parameter"), Amount::from(0.)),
                (ParameterName::new("Parameter2"), Amount::from(0.)),
            ]),
        )]);
        let state_hash = StateHash::new(&state);
        let mut possible_states = PossibleStates::new();
        possible_states
            .append_state(state_hash, state.clone())
            .unwrap();
        let expected = HashMap::from([(state_hash, state)]);
        assert_eq!(possible_states.0, expected);

        let new_state = State::new(vec![(
            EntityName::new("A"),
            Entity::new(vec![
                (ParameterName::new("Parameter"), Amount::from(1.)),
                (ParameterName::new("Parameter2"), Amount::from(2.)),
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
        let state_hash = StateHash::new(&State::default());
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
        let state_hash = StateHash::new(&State::default());
        let probability = Probability::from(0.2);
        reachable_states
            .append_state(state_hash, probability)
            .unwrap();
        let state_hash = StateHash::new(&State::new(vec![(
            EntityName::new("A"),
            Entity::new(vec![(ParameterName::new("Parameter"), Amount::from(0.))]),
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
        let state_hash = StateHash::new(&State::default());
        let probability = Probability::from(0.5);
        reachable_states
            .append_state(state_hash, probability)
            .unwrap();
        let state_hash = StateHash::new(&State::new(vec![(
            EntityName::new("A"),
            Entity::new(vec![(ParameterName::new("Parameter"), Amount::from(0.))]),
        )]));
        let probability = Probability::from(0.5);
        reachable_states
            .append_state(state_hash, probability)
            .unwrap();
        assert_eq!(reachable_states.entropy(), Entropy::from(1.));
    }

    #[test]
    fn euclidean_norm() {
        let mut reachable_states = ReachableStates::new();
        reachable_states
            .append_state(StateHash::from(1), Probability::new(0.5))
            .unwrap();
        reachable_states
            .append_state(StateHash::from(2), Probability::new(0.25))
            .unwrap();
        reachable_states
            .append_state(StateHash::from(3), Probability::new(0.25))
            .unwrap();
        let mut base_reachable_states = ReachableStates::new();
        base_reachable_states
            .append_state(StateHash::from(1), Probability::new(0.5))
            .unwrap();
        base_reachable_states
            .append_state(StateHash::from(2), Probability::new(0.5))
            .unwrap();
        assert_eq!(
            Entropy::new(0.3535533905932738),
            reachable_states.euclidean_norm(&base_reachable_states)
        );
    }
}
