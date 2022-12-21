use std::fmt::Display;
use std::hash::{Hash, Hasher};

#[allow(unused_imports)]
use hashbrown::{HashMap, HashSet};
#[allow(unused_imports)]
use itertools::Itertools;

use derive_more::*;

use crate::cache::*;
use crate::error::ResourceCapacityError;
use crate::resource::*;
use crate::state::*;
use crate::units::*;

/// An action a rule can take on a single entity and resource when its condition is met.
#[derive(PartialEq, Clone, Debug, Default)]
pub struct Action {
    resource: ResourceName,
    target: EntityName,
    amount: Amount,
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Action: set {} of {} to {}",
            self.resource, self.target, self.amount
        )
    }
}

impl Action {
    pub fn new() -> Self {
        Self {
            resource: ResourceName::new(),
            target: EntityName::new(),
            amount: Amount::new(),
        }
    }

    pub fn from(resource: ResourceName, target: EntityName, amount: Amount) -> Self {
        Self {
            resource,
            target,
            amount,
        }
    }

    pub fn target(&self) -> &EntityName {
        &self.target
    }

    pub fn resource(&self) -> &ResourceName {
        &self.resource
    }

    pub fn amount(&self) -> Amount {
        self.amount
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Display, Default, From, AsRef, AsMut, Into)]
pub struct ActionName(String);

impl ActionName {
    pub fn new() -> Self {
        Self("".to_string())
    }
}

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
)]
pub struct ProbabilityWeight(f64);

impl Hash for ProbabilityWeight {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialEq for ProbabilityWeight {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl From<f64> for ProbabilityWeight {
    fn from(value: f64) -> Self {
        if value < 0. {
            panic!("ProbabilityWeight cannot be negative");
        }
        Self(value)
    }
}

impl ProbabilityWeight {
    pub fn new() -> Self {
        Self(0.)
    }
}

/// An abstraction over the transition rates of the underlying markov chain.
#[derive(Clone, Debug)]
pub struct Rule {
    description: String,

    /// The conditions that must be met for the rule to be applied.
    condition: fn(State) -> RuleApplies,

    /// A measure of how often the rule is applied when the condition is met.
    ///
    /// As two rules cannot be applied at the same time, first, the probability that no rule applies is calculated.
    /// The remaining probability is divived among the remaining rules according to their weights.
    weight: ProbabilityWeight,

    /// A function which specifies to which state the rule leads when applied.
    ///
    /// The function takes the current state as input and returns multiple actions.
    /// A new state is then created by applying all actions to the current state.
    actions: fn(State) -> HashMap<ActionName, Action>,
}

impl Default for Rule {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Rule:")?;
        writeln!(f, "Description: {}", self.description)?;
        writeln!(f, "Weight: {}", self.weight)?;
        Ok(())
    }
}

impl Rule {
    pub fn new() -> Self {
        Self {
            description: "".to_string(),
            condition: |_| RuleApplies::from(false),
            weight: ProbabilityWeight::from(0.),
            actions: |_| HashMap::new(),
        }
    }
    pub fn from(
        description: String,
        condition: fn(State) -> RuleApplies,
        probability_weight: ProbabilityWeight,
        actions: fn(State) -> HashMap<ActionName, Action>,
    ) -> Self {
        Self {
            description,
            condition,
            weight: probability_weight,
            actions,
        }
    }

    /// Checks if a given rule applies to the given state using or updating the cache respectively.
    pub(crate) fn applies(
        &self,
        cache: &Cache,
        rule_name: RuleName,
        state: State,
    ) -> (RuleApplies, Option<ConditionCacheUpdate>) {
        if self.weight == ProbabilityWeight(0.) {
            return (RuleApplies(false), None);
        }
        let base_state_hash = StateHash::from_state(&state);
        match cache.condition(&rule_name, &base_state_hash) {
            Some(rule_applies) => (*rule_applies, None),
            None => {
                let rule_applies = (self.condition)(state);
                let cache = ConditionCacheUpdate::from(rule_name, base_state_hash, rule_applies);
                (rule_applies, Some(cache))
            }
        }
    }

    pub(crate) fn apply(
        &self,
        cache: &Cache,
        possible_states: &PossibleStates,
        rule_name: RuleName,
        base_state_hash: StateHash,
        base_state: State,
        resources: &HashMap<ResourceName, Resource>,
    ) -> Result<(State, Option<ActionCacheUpdate>), ResourceCapacityError> {
        match cache.action(&rule_name, &base_state_hash) {
            Some(new_state_hash) => Ok((
                possible_states
                    .state(&new_state_hash)
                    .expect("Cached new_state should be in possible states")
                    .clone(),
                None,
            )),
            None => {
                let actions = (self.actions)(base_state.clone());
                let new_state = base_state.apply_actions(actions);

                Resource::check_resource_capacities(resources, &new_state)?;

                let new_state_hash = StateHash::from_state(&new_state);
                let cache_update =
                    ActionCacheUpdate::from(rule_name.clone(), base_state_hash, new_state_hash);
                Ok((new_state, Some(cache_update)))
            }
        }
    }

    pub fn weight(&self) -> ProbabilityWeight {
        self.weight
    }

    pub fn description(&self) -> &String {
        &self.description
    }

    pub fn condition(&self) -> fn(State) -> RuleApplies {
        self.condition
    }

    pub fn actions(&self) -> fn(State) -> HashMap<ActionName, Action> {
        self.actions
    }
}

#[derive(
    Copy, Clone, PartialEq, Eq, Hash, Debug, Display, Default, From, Into, AsRef, AsMut, Not,
)]
pub struct RuleApplies(bool);

impl RuleApplies {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self(false)
    }

    pub fn is_true(&self) -> bool {
        self.0
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Display, Default, From, AsRef, AsMut, Into)]
pub struct RuleName(String);

impl RuleName {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self("".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_should_return_empty_cache_update_on_found_cache() {
        let mut cache = Cache::new();
        let rule = Rule::from(
            "Only for testing purposes".to_string(),
            |_| RuleApplies(true),
            ProbabilityWeight(1.),
            |_| HashMap::new(),
        );
        let rule_name = RuleName("Test".to_string());
        let state = State::new();
        let state_hash = StateHash::from_state(&state);
        cache
            .add_condition(rule_name.clone(), state_hash, RuleApplies(true))
            .unwrap();
        let (rule_applies, cache_update) = rule.applies(&cache, rule_name, state);
        assert_eq!(rule_applies, RuleApplies(true));
        assert_eq!(cache_update, None);
    }

    #[test]
    fn applies_should_return_proper_cache_update_on_missing_cache() {
        let cache = Cache::new();
        let rule = Rule::from(
            "Only for testing purposes".to_string(),
            |_| RuleApplies(true),
            ProbabilityWeight(1.),
            |_| HashMap::new(),
        );
        let rule_name = RuleName("Test".to_string());
        let state = State::new();
        let (rule_applies, cache_update) = rule.applies(&cache, rule_name, state.clone());
        assert_eq!(rule_applies, RuleApplies(true));
        assert_eq!(
            cache_update,
            Some(ConditionCacheUpdate::from(
                RuleName("Test".to_string()),
                StateHash::from_state(&state),
                RuleApplies(true),
            ))
        );
    }

    #[test]
    fn apply_should_return_empty_cache_update_on_found_cache() {
        let mut cache = Cache::new();
        let rule = Rule::from(
            "Only for testing purposes".to_string(),
            |_| RuleApplies(true),
            ProbabilityWeight(1.),
            |_| HashMap::new(),
        );
        let rule_name = RuleName("Test".to_string());
        let state = State::new();
        let state_hash = StateHash::from_state(&state);
        let mut possible_states = PossibleStates::new();
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
                &HashMap::new(),
            )
            .unwrap();
        assert_eq!(new_state, state);
        assert_eq!(cache_update, None);
    }

    #[test]
    fn apply_should_return_proper_cache_update_on_missing_cache() {
        let cache = Cache::new();
        let rule = Rule::from(
            "Only for testing purposes".to_string(),
            |_| RuleApplies(true),
            ProbabilityWeight(1.),
            |_| HashMap::new(),
        );
        let rule_name = RuleName("Test".to_string());
        let state = State::new();
        let state_hash = StateHash::from_state(&state);
        let possible_states = PossibleStates::new();
        let (new_state, cache_update) = rule
            .apply(
                &cache,
                &possible_states,
                rule_name,
                state_hash,
                state.clone(),
                &HashMap::new(),
            )
            .unwrap();
        assert_eq!(new_state, state);
        assert_eq!(
            cache_update,
            Some(ActionCacheUpdate::from(
                RuleName("Test".to_string()),
                StateHash::from_state(&state),
                StateHash::from_state(&state),
            ))
        );
    }
}
