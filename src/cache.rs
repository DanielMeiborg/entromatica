#[allow(unused_imports)]
use hashbrown::{HashMap, HashSet};
#[allow(unused_imports)]
use itertools::Itertools;

use crate::rules::*;
use crate::state::*;

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub(crate) struct RuleCache {
    pub condition: HashMap<StateHash, RuleApplies>,
    pub actions: HashMap<StateHash, StateHash>,
}

impl RuleCache {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            condition: HashMap::new(),
            actions: HashMap::new(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub(crate) struct Cache {
    pub rules: HashMap<RuleName, RuleCache>,
}

impl Cache {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub(crate) struct ConditionCacheUpdate {
    pub rule_name: RuleName,
    pub base_state_hash: StateHash,
    pub applies: RuleApplies,
}

impl ConditionCacheUpdate {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            rule_name: RuleName::new(),
            base_state_hash: StateHash::new(),
            applies: RuleApplies::new(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub(crate) struct ActionCacheUpdate {
    pub rule_name: RuleName,
    pub base_state_hash: StateHash,
    pub new_state_hash: StateHash,
}

impl ActionCacheUpdate {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            rule_name: RuleName::new(),
            base_state_hash: StateHash::new(),
            new_state_hash: StateHash::new(),
        }
    }
}
