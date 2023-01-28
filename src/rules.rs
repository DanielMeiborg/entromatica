use std::{
    fmt::{Debug, Display},
    hash::{Hash, Hasher},
    sync::Arc,
};

use backtrace::Backtrace as trc;
use derive_more::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::prelude::*;

#[derive(
    PartialOrd,
    Clone,
    Copy,
    Default,
    Debug,
    Display,
    Into,
    AsRef,
    AsMut,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Sum,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
    Serialize,
    Deserialize,
)]
pub struct ProbabilityWeight(f64);

impl Hash for ProbabilityWeight {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialEq for ProbabilityWeight {
    fn eq(&self, other: &ProbabilityWeight) -> bool {
        self.0 == other.0
    }
}

impl From<f64> for ProbabilityWeight {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl ProbabilityWeight {
    pub fn new() -> Self {
        Self(0.)
    }
}

#[derive(From, Into, Clone)]
pub struct Rule<T> {
    description: String,
    condition: Arc<dyn Fn(State<T>) -> RuleApplies + Send + Sync>,
    weight: ProbabilityWeight,
    action: Arc<dyn Fn(State<T>) -> State<T> + Send + Sync>,
}

impl<T: Debug> Debug for Rule<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Rule:")?;
        writeln!(f, "Description: {}", self.description)?;
        writeln!(f, "Weight: {}", self.weight)?;
        Ok(())
    }
}

impl<T> Display for Rule<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Rule:")?;
        writeln!(f, "Description: {}", self.description)?;
        writeln!(f, "Weight: {}", self.weight)?;
        Ok(())
    }
}

impl<T> Rule<T>
where
    T: Clone
        + Debug
        + Hash
        + PartialEq
        + Default
        + Serialize
        + Send
        + Sync
        + for<'a> Deserialize<'a>,
{
    pub fn new(
        description: String,
        condition: Arc<dyn Fn(State<T>) -> RuleApplies + Send + Sync>,
        probability_weight: ProbabilityWeight,
        action: Arc<dyn Fn(State<T>) -> State<T> + Send + Sync>,
    ) -> Self {
        Self {
            description,
            condition,
            weight: probability_weight,
            action,
        }
    }

    pub(crate) fn applies(
        &self,
        cache: &Cache,
        rule_name: RuleName,
        state: State<T>,
    ) -> Result<(RuleApplies, Option<ConditionCacheUpdate>), CacheError> {
        if self.weight == ProbabilityWeight(0.) {
            return Ok((RuleApplies(false), None));
        }
        let base_state_hash = StateHash::new(&state);
        if cache.contains_condition(&rule_name, &base_state_hash)? {
            Ok((*cache.condition(&rule_name, &base_state_hash)?, None))
        } else {
            let rule_applies = (self.condition)(state);
            let condition_cache_update =
                ConditionCacheUpdate::new(rule_name, base_state_hash, rule_applies);
            Ok((rule_applies, Some(condition_cache_update)))
        }
    }

    pub(crate) fn apply(
        &self,
        cache: &Cache,
        possible_states: &PossibleStates<T>,
        rule_name: RuleName,
        base_state_hash: StateHash,
        base_state: State<T>,
    ) -> Result<(State<T>, Option<ActionCacheUpdate>), ErrorKind<T>> {
        if cache.contains_action(&rule_name, &base_state_hash)? {
            Ok((
                possible_states
                    .state(&cache.action(&rule_name, &base_state_hash)?)
                    .map_err(ErrorKind::PossibleStatesError)?
                    .clone(),
                None,
            ))
        } else {
            let new_state = (self.action)(base_state);

            let new_state_hash = StateHash::new(&new_state);
            let condition_cache_update =
                ActionCacheUpdate::new(rule_name, base_state_hash, new_state_hash);
            Ok((new_state, Some(condition_cache_update)))
        }
    }

    pub fn weight(&self) -> ProbabilityWeight {
        self.weight
    }

    pub fn description(&self) -> &String {
        &self.description
    }

    pub fn condition(&self) -> &(dyn Fn(State<T>) -> RuleApplies + Send + Sync) {
        &*self.condition
    }

    pub fn action(&self) -> &(dyn Fn(State<T>) -> State<T> + Send + Sync) {
        &*self.action
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Error)]
pub enum RuleError {
    #[error("Rule not found: {rule_name:#?}")]
    RuleNotFound { rule_name: RuleName, context: trc },

    #[error("Rule already exists: {rule_name:#?}")]
    RuleAlreadyExists { rule_name: RuleName, context: trc },
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
    Not,
    Serialize,
    Deserialize,
)]
pub struct RuleApplies(bool);

impl RuleApplies {
    pub fn new(applies: bool) -> Self {
        Self(applies)
    }

    pub fn is_true(&self) -> bool {
        self.0
    }

    pub fn applies(&self) -> bool {
        self.0
    }
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
    AsRef,
    AsMut,
    Into,
    Serialize,
    Deserialize,
)]
pub struct RuleName(String);

impl RuleName {
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_should_return_empty_cache_update_on_found_cache() {
        let mut cache = Cache::new();
        let rule = Rule::new(
            "Only for testing purposes".to_string(),
            Arc::new(|_: State<i32>| RuleApplies(true)),
            ProbabilityWeight(1.),
            Arc::new(|state: State<i32>| state),
        );
        let rule_name = RuleName::new("Test");
        let state = State::default();
        let state_hash = StateHash::new(&state);
        cache
            .add_condition(rule_name.clone(), state_hash, RuleApplies(true))
            .unwrap();
        let (rule_applies, cache_update) = rule.applies(&cache, rule_name, state).unwrap();
        assert_eq!(rule_applies, RuleApplies(true));
        assert_eq!(cache_update, None);
    }

    #[test]
    fn applies_should_return_proper_cache_update_on_missing_cache() {
        let cache = Cache::new();
        let rule = Rule::new(
            "Only for testing purposes".to_string(),
            Arc::new(|_: State<i32>| RuleApplies(true)),
            ProbabilityWeight(1.),
            Arc::new(|state: State<i32>| state),
        );
        let rule_name = RuleName::new("Test");
        let state = State::default();
        let (rule_applies, cache_update) = rule.applies(&cache, rule_name, state.clone()).unwrap();
        assert_eq!(rule_applies, RuleApplies(true));
        assert_eq!(
            cache_update,
            Some(ConditionCacheUpdate::new(
                RuleName::new("Test"),
                StateHash::new(&state),
                RuleApplies(true),
            ))
        );
    }

    #[test]
    fn apply_should_return_empty_cache_update_on_found_cache() {
        let mut cache = Cache::new();
        let rule = Rule::new(
            "Only for testing purposes".to_string(),
            Arc::new(|_: State<i32>| RuleApplies(true)),
            ProbabilityWeight(1.),
            Arc::new(|state: State<i32>| state),
        );
        let rule_name = RuleName::new("Test");
        let state = State::default();
        let state_hash = StateHash::new(&state);
        let mut possible_states = PossibleStates::default();
        possible_states
            .append_state(state_hash, state.clone())
            .unwrap();
        cache
            .add_action(rule_name.clone(), state_hash, state_hash)
            .unwrap();
        let (new_state, cache_update) = rule
            .apply(
                &cache,
                &possible_states,
                rule_name,
                state_hash,
                state.clone(),
            )
            .unwrap();
        assert_eq!(new_state, state);
        assert_eq!(cache_update, None);
    }

    #[test]
    fn apply_should_return_proper_cache_update_on_missing_cache() {
        let cache = Cache::new();
        let rule = Rule::new(
            "Only for testing purposes".to_string(),
            Arc::new(|_: State<i32>| RuleApplies(true)),
            ProbabilityWeight(1.),
            Arc::new(|state: State<i32>| state),
        );
        let rule_name = RuleName::new("Test");
        let state = State::default();
        let state_hash = StateHash::new(&state);
        let possible_states = PossibleStates::default();
        let (new_state, cache_update) = rule
            .apply(
                &cache,
                &possible_states,
                rule_name,
                state_hash,
                state.clone(),
            )
            .unwrap();
        assert_eq!(new_state, state);
        assert_eq!(
            cache_update,
            Some(ActionCacheUpdate::new(
                RuleName::new("Test"),
                StateHash::new(&state),
                StateHash::new(&state),
            ))
        );
    }
}
