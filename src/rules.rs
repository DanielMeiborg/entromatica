use std::hash::{Hash, Hasher};

#[allow(unused_imports)]
use hashbrown::{HashMap, HashSet};
#[allow(unused_imports)]
use itertools::Itertools;

use derive_more::*;

use crate::cache::*;
use crate::resources::*;
use crate::state::*;
use crate::units::*;

/// An action a rule can take on a single entity and resource when its condition is met.
#[derive(PartialEq, Clone, Debug, Default)]
pub struct Action {
    // TODO: name to description i.e. vec to hashmap
    pub name: String,
    pub resource: ResourceName,
    pub entity_name: EntityName,
    pub new_amount: Amount,
}

impl Action {
    pub fn new() -> Self {
        Self {
            name: "".to_string(),
            resource: ResourceName::new(),
            entity_name: EntityName::new(),
            new_amount: Amount::new(),
        }
    }
}

#[derive(
    PartialOrd,
    Clone,
    Copy,
    Default,
    Debug,
    Display,
    From,
    Into,
    AsRef,
    AsMut,
    Deref,
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
pub struct ProbabilityWeight(pub f64);

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

impl ProbabilityWeight {
    pub fn new() -> Self {
        Self(0.)
    }
}

/// An abstraction over the transition rates of the underlying markov chain.
#[derive(Clone, Debug)]
pub struct Rule {
    pub description: String,

    /// The conditions that must be met for the rule to be applied.
    pub condition: fn(State) -> RuleApplies,

    /// A measure of how often the rule is applied when the condition is met.
    ///
    /// As two rules cannot be applied at the same time, first, the probability that no rule applies is calculated.
    /// The remaining probability is divived among the remaining rules according to their weights.
    pub probability_weight: ProbabilityWeight,

    /// A function which specifies to which state the rule leads when applied.
    ///
    /// The function takes the current state as input and returns multiple actions.
    /// A new state is then created by applying all actions to the current state.
    pub actions: fn(State) -> Vec<Action>,
}

impl Rule {
    #[allow(dead_code)]
    pub fn new(
        description: String,
        condition: fn(State) -> RuleApplies,
        probability_weight: ProbabilityWeight,
        actions: fn(State) -> Vec<Action>,
    ) -> Self {
        Self {
            description,
            condition,
            probability_weight,
            actions,
        }
    }

    fn applies(&self, state: State) -> RuleApplies {
        (self.condition)(state)
    }

    fn apply(&self, state: State) -> State {
        let actions = (self.actions)(state.clone());
        state.apply_actions(actions)
    }

    /// Checks if a given rule applies to the given state using or updating the cache respectively.
    pub(crate) fn applies_using_cache(
        &self,
        cache: &Cache,
        rule_name: RuleName,
        state: State,
    ) -> (RuleApplies, Option<ConditionCacheUpdate>) {
        if self.probability_weight == ProbabilityWeight(0.) {
            return (RuleApplies(false), None);
        }
        let base_state_hash = StateHash::from_state(&state);
        let rule_cache = cache
            .rules
            .get(&rule_name)
            .expect("Rule {rule_name} not found in cache");
        match rule_cache.condition.get(&base_state_hash) {
            Some(rule_applies) => (*rule_applies, None),
            None => {
                let result = self.applies(state);
                let cache = ConditionCacheUpdate {
                    rule_name,
                    base_state_hash,
                    applies: result,
                };
                (result, Some(cache))
            }
        }
    }

    pub(crate) fn apply_using_cache(
        &self,
        cache: &Cache,
        possible_states: &PossibleStates,
        rule_name: RuleName,
        base_state_hash: StateHash,
        base_state: State,
        resources: &HashMap<ResourceName, Resource>,
    ) -> (State, Option<ActionCacheUpdate>) {
        let rule_cache = cache
            .rules
            .get(&rule_name)
            .expect("Rule {rule_name} not found in cache");

        if let Some(new_state_hash) = rule_cache.actions.get(&base_state_hash) {
            return (
                possible_states
                    .get(new_state_hash)
                    .expect("Cached new_state should be in possible states")
                    .clone(),
                None,
            );
        }
        let new_state = self.apply(base_state);

        Resource::assert_resource_capacities(resources, &new_state);

        let new_state_hash = StateHash::from_state(&new_state);
        let cache_update = ActionCacheUpdate {
            rule_name: rule_name.clone(),
            base_state_hash,
            new_state_hash,
        };
        (new_state, Some(cache_update))
    }
}

#[derive(
    Copy, Clone, PartialEq, Eq, Hash, Debug, Display, Default, From, Into, AsRef, AsMut, Not,
)]
pub struct RuleApplies(pub bool);

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
pub struct RuleName(pub String);

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
    #[should_panic]
    fn applies_using_cache_should_panic_on_incomplete_cache() {
        let cache = Cache::new();
        let rule = Rule::new(
            "Only for testing purposes".to_string(),
            |_| RuleApplies(true),
            ProbabilityWeight(1.),
            |_| vec![],
        );
        let rule_name = RuleName("Test".to_string());
        let state = State::new();
        rule.applies_using_cache(&cache, rule_name, state);
    }

    #[test]
    fn applies_using_cache_should_return_empty_cache_update_on_found_cache() {
        let mut cache = Cache::new();
        let rule = Rule::new(
            "Only for testing purposes".to_string(),
            |_| RuleApplies(true),
            ProbabilityWeight(1.),
            |_| vec![],
        );
        let rule_name = RuleName("Test".to_string());
        let state = State::new();
        let state_hash = StateHash::from_state(&state);
        cache.rules.insert(rule_name.clone(), RuleCache::new());
        cache
            .rules
            .get_mut(&rule_name)
            .unwrap()
            .condition
            .insert(state_hash, RuleApplies(true));
        let (result, cache_update) = rule.applies_using_cache(&cache, rule_name, state);
        assert_eq!(result, RuleApplies(true));
        assert_eq!(cache_update, None);
    }

    #[test]
    fn applies_using_cache_should_return_proper_cache_update_on_missing_cache() {
        let mut cache = Cache::new();
        let rule = Rule::new(
            "Only for testing purposes".to_string(),
            |_| RuleApplies(true),
            ProbabilityWeight(1.),
            |_| vec![],
        );
        let rule_name = RuleName("Test".to_string());
        cache
            .rules
            .insert(RuleName("Test".to_string()), RuleCache::new());
        let state = State::new();
        let (result, cache_update) = rule.applies_using_cache(&cache, rule_name, state.clone());
        assert_eq!(result, RuleApplies(true));
        assert_eq!(
            cache_update,
            Some(ConditionCacheUpdate {
                rule_name: RuleName("Test".to_string()),
                base_state_hash: StateHash::from_state(&state),
                applies: RuleApplies(true),
            })
        );
    }

    #[test]
    #[should_panic]
    fn apply_using_cache_should_panic_on_incomplete_cache() {
        let cache = Cache::new();
        let rule = Rule::new(
            "Only for testing purposes".to_string(),
            |_| RuleApplies(true),
            ProbabilityWeight(1.),
            |_| vec![],
        );
        let rule_name = RuleName("Test".to_string());
        let state = State::new();
        let state_hash = StateHash::from_state(&state);
        let possible_states = PossibleStates::new();
        rule.apply_using_cache(
            &cache,
            &possible_states,
            rule_name,
            state_hash,
            state,
            &HashMap::new(),
        );
    }

    #[test]
    fn apply_using_cache_should_return_empty_cache_update_on_found_cache() {
        let mut cache = Cache::new();
        let rule = Rule::new(
            "Only for testing purposes".to_string(),
            |_| RuleApplies(true),
            ProbabilityWeight(1.),
            |_| vec![],
        );
        let rule_name = RuleName("Test".to_string());
        let state = State::new();
        let state_hash = StateHash::from_state(&state);
        let mut possible_states = PossibleStates::new();
        possible_states.append_state(state_hash, state.clone()).unwrap();
        cache.rules.insert(rule_name.clone(), RuleCache::new());
        cache
            .rules
            .get_mut(&rule_name)
            .unwrap()
            .actions
            .insert(state_hash, state_hash);
        let (result, cache_update) = rule.apply_using_cache(
            &cache,
            &possible_states,
            rule_name,
            state_hash,
            state.clone(),
            &HashMap::new(),
        );
        assert_eq!(result, state);
        assert_eq!(cache_update, None);
    }

    #[test]
    fn apply_using_cache_should_return_proper_cache_update_on_missing_cache() {
        let mut cache = Cache::new();
        let rule = Rule::new(
            "Only for testing purposes".to_string(),
            |_| RuleApplies(true),
            ProbabilityWeight(1.),
            |_| vec![],
        );
        let rule_name = RuleName("Test".to_string());
        let state = State::new();
        let state_hash = StateHash::from_state(&state);
        let possible_states = PossibleStates::new();
        cache
            .rules
            .insert(RuleName("Test".to_string()), RuleCache::new());
        let (result, cache_update) = rule.apply_using_cache(
            &cache,
            &possible_states,
            rule_name,
            state_hash,
            state.clone(),
            &HashMap::new(),
        );
        assert_eq!(result, state);
        assert_eq!(
            cache_update,
            Some(ActionCacheUpdate {
                rule_name: RuleName("Test".to_string()),
                base_state_hash: StateHash::from_state(&state),
                new_state_hash: StateHash::from_state(&state),
            })
        );
    }
}
