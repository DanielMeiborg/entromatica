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
    pub entity: EntityName,
    pub new_amount: Amount,
}

impl Action {
    pub fn new() -> Self {
        Self {
            name: "".to_string(),
            resource: ResourceName::new(),
            entity: EntityName::new(),
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

    pub fn to_bits(self) -> u64 {
        self.0.to_bits()
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

    pub fn applies(&self, state: State) -> RuleApplies {
        (self.condition)(state)
    }

    pub fn apply(&self, state: State, resources: &HashMap<ResourceName, Resource>) -> State {
        let actions = (self.actions)(state.clone());
        state.apply_actions(&actions, resources)
    }

    /// Checks if a given rule applies to the given state using or updating the cache respectively.
    pub(crate) fn applies_using_cache(
        &self,
        cache: &Cache,
        rule_name: RuleName,
        base_state_hash: StateHash,
        state: State,
    ) -> (RuleApplies, Option<ConditionCacheUpdate>) {
        if self.probability_weight == ProbabilityWeight(0.) {
            return (RuleApplies(false), None);
        }
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
