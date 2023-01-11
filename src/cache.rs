use std::fmt::Display;
use std::fmt::Formatter;

#[allow(unused_imports)]
use hashbrown::{HashMap, HashSet};
#[allow(unused_imports)]
use itertools::Itertools;

use backtrace::Backtrace as trc;
use petgraph::graph::NodeIndex;
use petgraph::Graph;
use thiserror::Error;

use crate::*;

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub(self) struct RuleCache {
    condition: HashMap<StateHash, RuleApplies>,
    actions: HashMap<StateHash, StateHash>,
}

impl Display for RuleCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "RuleCache:")?;
        for (base_state_hash, applies) in &self.condition {
            if applies.is_true() {
                match self.condition(base_state_hash) {
                    Ok(new_state_hash) => {
                        writeln!(f, "Rule applies for {base_state_hash} -> {new_state_hash}")?
                    }
                    Err(error) => return error.fmt(f),
                };
            } else {
                writeln!(f, "Rule does not apply for {base_state_hash}")?;
            }
        }
        Ok(())
    }
}

impl RuleCache {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            condition: HashMap::new(),
            actions: HashMap::new(),
        }
    }

    pub fn condition(&self, base_state_hash: &StateHash) -> Result<&RuleApplies, RuleCacheError> {
        self.condition
            .get(base_state_hash)
            .ok_or_else(|| RuleCacheError::ConditionNotFound {
                base_state_hash: *base_state_hash,
                context: get_backtrace(),
            })
    }

    pub fn action(&self, base_state_hash: &StateHash) -> Result<&StateHash, RuleCacheError> {
        self.actions
            .get(base_state_hash)
            .ok_or_else(|| RuleCacheError::ActionNotFound {
                base_state_hash: *base_state_hash,
                context: get_backtrace(),
            })
    }

    pub fn add_condition(
        &mut self,
        base_state_hash: StateHash,
        applies: RuleApplies,
    ) -> Result<(), RuleCacheError> {
        if self.condition.contains_key(&base_state_hash) {
            if self.condition.get(&base_state_hash) == Some(&applies) {
                return Ok(());
            } else {
                return Err(RuleCacheError::ConditionAlreadyExists {
                    base_state_hash,
                    applies,
                    context: get_backtrace(),
                });
            }
        }
        self.condition.insert(base_state_hash, applies);
        Ok(())
    }

    pub fn add_action(
        &mut self,
        base_state_hash: StateHash,
        new_state_hash: StateHash,
    ) -> Result<(), RuleCacheError> {
        if self.actions.contains_key(&base_state_hash) {
            if self.actions.get(&base_state_hash) == Some(&new_state_hash) {
                return Ok(());
            } else {
                return Err(RuleCacheError::ActionAlreadyExists {
                    base_state_hash,
                    new_state_hash,
                    context: get_backtrace(),
                });
            }
        }
        self.actions.insert(base_state_hash, new_state_hash);
        Ok(())
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Error)]
pub(self) enum RuleCacheError {
    #[error("Condition already exists: {base_state_hash:#?} -> {applies:#?}")]
    ConditionAlreadyExists {
        base_state_hash: StateHash,
        applies: RuleApplies,
        context: trc,
    },

    #[error("Action already exists: {base_state_hash:#?} -> {new_state_hash:#?}")]
    ActionAlreadyExists {
        base_state_hash: StateHash,
        new_state_hash: StateHash,
        context: trc,
    },

    #[error("Condition not found: {base_state_hash:#?}")]
    ConditionNotFound {
        base_state_hash: StateHash,
        context: trc,
    },

    #[error("Action not found: {base_state_hash:#?}")]
    ActionNotFound {
        base_state_hash: StateHash,
        context: trc,
    },
}

#[derive(Debug, Clone, Error)]
#[error(transparent)]
pub(crate) struct InternalCacheError(#[from] RuleCacheError);

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub(crate) struct Cache {
    rules: HashMap<RuleName, RuleCache>,
}

impl Display for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Cache:")?;
        for (rule_name, rule_cache) in &self.rules {
            writeln!(f, "{rule_name}: {rule_cache}")?;
        }
        Ok(())
    }
}

impl Cache {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    pub(self) fn rule(&self, rule_name: &RuleName) -> Result<&RuleCache, CacheError> {
        self.rules
            .get(rule_name)
            .ok_or_else(|| CacheError::RuleNotFound {
                rule_name: rule_name.clone(),
                context: get_backtrace(),
            })
    }

    pub(self) fn rule_mut(&mut self, rule_name: &RuleName) -> Result<&mut RuleCache, CacheError> {
        self.rules
            .get_mut(rule_name)
            .ok_or_else(|| CacheError::RuleNotFound {
                rule_name: rule_name.clone(),
                context: get_backtrace(),
            })
    }

    pub(self) fn add_rule(&mut self, rule_name: RuleName) -> Result<(), CacheError> {
        if self.rules.contains_key(&rule_name) {
            return Err(CacheError::RuleAlreadyExists {
                rule_name,
                context: get_backtrace(),
            });
        }
        self.rules.insert(rule_name, RuleCache::new());
        Ok(())
    }

    pub fn condition(
        &self,
        rule_name: &RuleName,
        base_state_hash: &StateHash,
    ) -> Result<&RuleApplies, CacheError> {
        Ok(self.rule(rule_name)?.condition(base_state_hash)?)
    }

    pub fn contains_condition(
        &self,
        rule_name: &RuleName,
        base_state_hash: &StateHash,
    ) -> Result<bool, CacheError> {
        match self.rule(rule_name) {
            Ok(rule_cache) => Ok(rule_cache.condition.contains_key(base_state_hash)),
            Err(CacheError::RuleNotFound { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    pub fn contains_action(
        &self,
        rule_name: &RuleName,
        base_state_hash: &StateHash,
    ) -> Result<bool, CacheError> {
        match self.rule(rule_name) {
            Ok(rule_cache) => Ok(rule_cache.actions.contains_key(base_state_hash)),
            Err(CacheError::RuleNotFound { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    pub fn action(
        &self,
        rule_name: &RuleName,
        base_state_hash: &StateHash,
    ) -> Result<StateHash, CacheError> {
        Ok(*self.rule(rule_name)?.action(base_state_hash)?)
    }

    pub fn add_action(
        &mut self,
        rule_name: RuleName,
        base_state_hash: StateHash,
        new_state_hash: StateHash,
    ) -> Result<(), CacheError> {
        match self.rule_mut(&rule_name) {
            Ok(rule_cache) => Ok(rule_cache.add_action(base_state_hash, new_state_hash)?),
            Err(cache_error) => {
                if let CacheError::RuleNotFound { rule_name, .. } = cache_error {
                    self.add_rule(rule_name.clone())?;
                    let rule_cache = self.rule_mut(&rule_name)?;
                    Ok(rule_cache.add_action(base_state_hash, new_state_hash)?)
                } else {
                    Err(cache_error)
                }
            }
        }
    }

    pub fn add_condition(
        &mut self,
        rule_name: RuleName,
        base_state_hash: StateHash,
        applies: RuleApplies,
    ) -> Result<(), CacheError> {
        match self.rule_mut(&rule_name) {
            Ok(rule_cache) => Ok(rule_cache.add_condition(base_state_hash, applies)?),
            Err(cache_error) => {
                if let CacheError::RuleNotFound { rule_name, .. } = cache_error {
                    self.add_rule(rule_name.clone())?;
                    let rule_cache = self.rule_mut(&rule_name)?;
                    Ok(rule_cache.add_condition(base_state_hash, applies)?)
                } else {
                    Err(cache_error)
                }
            }
        }
    }

    pub fn apply_condition_update(
        &mut self,
        update: ConditionCacheUpdate,
    ) -> Result<(), CacheError> {
        self.add_condition(update.rule_name, update.base_state_hash, update.applies)
    }

    pub fn apply_action_update(&mut self, update: ActionCacheUpdate) -> Result<(), CacheError> {
        self.add_action(
            update.rule_name,
            update.base_state_hash,
            update.new_state_hash,
        )
    }

    // TODO: Make secure
    ///Gets a graph from the possible states with the nodes being the states and the directed edges being the rule names.
    pub fn graph(&self, possible_states: PossibleStates) -> Graph<State, RuleName> {
        let mut graph = Graph::<State, RuleName>::new();
        let mut nodes: HashMap<StateHash, NodeIndex> = HashMap::new();
        for (state_hash, state) in possible_states.iter() {
            let node_index = graph.add_node(state.clone());
            nodes.insert(*state_hash, node_index);
        }
        for (state_hash, state_node) in &nodes {
            for (rule_name, rule_cache) in self.rules.iter() {
                if rule_cache.condition(state_hash).is_ok() {
                    if let Ok(new_state_hash) = rule_cache.action(state_hash) {
                        let new_state_node = nodes.get(new_state_hash).unwrap();
                        graph.add_edge(*state_node, *new_state_node, rule_name.clone());
                    }
                }
            }
        }
        graph
    }

    pub fn merge(&mut self, cache: &Self) -> Result<(), CacheError> {
        for (rule_name, rule_cache) in cache.rules.iter() {
            for (base_state_hash, applies) in rule_cache.condition.iter() {
                self.add_condition(rule_name.clone(), *base_state_hash, *applies)?;
            }
            for (base_state_hash, new_state_hash) in rule_cache.actions.iter() {
                self.add_action(rule_name.clone(), *base_state_hash, *new_state_hash)?;
            }
        }
        Ok(())
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Error)]
pub(crate) enum CacheError {
    #[error("Rule already exists: {rule_name:#?}")]
    RuleAlreadyExists { rule_name: RuleName, context: trc },

    #[error("Rule not found: {rule_name:#?}")]
    RuleNotFound { rule_name: RuleName, context: trc },

    #[error("Internal cache error: {source:#?}")]
    InternalError {
        #[source]
        source: InternalCacheError,
        context: trc,
    },
}

impl From<RuleCacheError> for CacheError {
    fn from(source: RuleCacheError) -> Self {
        Self::InternalError {
            source: InternalCacheError(source),
            context: get_backtrace(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub(crate) struct ConditionCacheUpdate {
    pub(self) rule_name: RuleName,
    pub(self) base_state_hash: StateHash,
    pub(self) applies: RuleApplies,
}

impl Display for ConditionCacheUpdate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ConditionCacheUpdate for base state {}: rule {} applies: {}",
            self.base_state_hash, self.rule_name, self.applies
        )
    }
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

    pub fn from(rule_name: RuleName, base_state_hash: StateHash, applies: RuleApplies) -> Self {
        Self {
            rule_name,
            base_state_hash,
            applies,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub(crate) struct ActionCacheUpdate {
    pub(self) rule_name: RuleName,
    pub(self) base_state_hash: StateHash,
    pub(self) new_state_hash: StateHash,
}

impl Display for ActionCacheUpdate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ActionCacheUpdate for base state {}: rule {} new state: {}",
            self.base_state_hash, self.rule_name, self.new_state_hash
        )
    }
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

    pub fn from(
        rule_name: RuleName,
        base_state_hash: StateHash,
        new_state_hash: StateHash,
    ) -> Self {
        Self {
            rule_name,
            base_state_hash,
            new_state_hash,
        }
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    fn example_cache() -> Cache {
        let mut cache = Cache::new();
        let rule_name = RuleName::from("test".to_string());
        let base_state_hash = StateHash::new();
        let new_state_hash = StateHash::new();
        let applies = RuleApplies::from(true);
        cache
            .add_condition(rule_name.clone(), base_state_hash, applies)
            .unwrap();
        cache
            .add_action(rule_name, base_state_hash, new_state_hash)
            .unwrap();
        cache
    }

    #[test]
    fn new() {
        let cache = Cache::new();
        assert_eq!(cache.rules.len(), 0);
    }

    #[test]
    fn contains_condition() {
        let cache = example_cache();
        assert!(cache
            .contains_condition(&RuleName::from("test".to_string()), &StateHash::new())
            .unwrap());
    }

    #[test]
    fn contains_action() {
        let cache = example_cache();
        assert!(cache
            .contains_action(&RuleName::from("test".to_string()), &StateHash::new())
            .unwrap());
    }

    #[test]
    fn action() {
        let cache = example_cache();
        assert_eq!(
            cache
                .action(&RuleName::from("test".to_string()), &StateHash::new())
                .unwrap(),
            StateHash::new()
        );
        assert!(matches!(
            cache
                .action(
                    &RuleName::from("nonexistium".to_string()),
                    &StateHash::new()
                )
                .unwrap_err(),
            CacheError::RuleNotFound { .. }
        ));
    }

    #[test]
    fn add_action() {
        let mut cache = example_cache();
        let base_state_hash = StateHash::from_state(&State::from_entities(vec![(
            EntityName::from("test".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("test".to_string()),
                Amount::from(1.),
            )]),
        )]));
        cache
            .add_action(
                RuleName::from("test".to_string()),
                base_state_hash,
                StateHash::new(),
            )
            .unwrap();
        assert_eq!(
            cache
                .action(&RuleName::from("test".to_string()), &base_state_hash)
                .unwrap(),
            StateHash::new()
        );
        assert!(matches!(
            cache
                .add_action(
                    RuleName::from("test".to_string()),
                    base_state_hash,
                    base_state_hash
                )
                .unwrap_err(),
            CacheError::InternalError { .. }
        ));
        assert_eq!(
            cache
                .action(&RuleName::from("test".to_string()), &base_state_hash)
                .unwrap(),
            StateHash::new()
        );
    }

    #[test]
    fn add_condition() {
        let mut cache = example_cache();
        let base_state_hash = StateHash::from_state(&State::from_entities(vec![(
            EntityName::from("test".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("test".to_string()),
                Amount::from(1.),
            )]),
        )]));
        cache
            .add_condition(
                RuleName::from("test".to_string()),
                base_state_hash,
                RuleApplies::from(true),
            )
            .unwrap();
        assert!(cache
            .contains_condition(&RuleName::from("test".to_string()), &base_state_hash)
            .unwrap());
        cache
            .add_condition(
                RuleName::from("another".to_string()),
                base_state_hash,
                RuleApplies::from(true),
            )
            .unwrap();
        assert!(cache
            .contains_condition(&RuleName::from("another".to_string()), &base_state_hash)
            .unwrap());
        assert!(matches!(
            cache
                .add_condition(
                    RuleName::from("test".to_string()),
                    base_state_hash,
                    RuleApplies::from(false)
                )
                .unwrap_err(),
            CacheError::InternalError { .. }
        ));
        assert!(cache
            .contains_condition(&RuleName::from("test".to_string()), &base_state_hash)
            .unwrap());
    }

    #[test]
    fn apply_condition_update() {
        let mut cache = example_cache();
        let base_state_hash = StateHash::from_state(&State::from_entities(vec![(
            EntityName::from("test".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("test".to_string()),
                Amount::from(1.),
            )]),
        )]));
        let update = ConditionCacheUpdate::from(
            RuleName::from("test".to_string()),
            base_state_hash,
            RuleApplies::from(true),
        );
        cache.apply_condition_update(update).unwrap();
        assert!(cache
            .contains_condition(&RuleName::from("test".to_string()), &base_state_hash)
            .unwrap());
    }

    #[test]
    fn apply_action_update() {
        let mut cache = example_cache();
        let base_state_hash = StateHash::from_state(&State::from_entities(vec![(
            EntityName::from("test".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("test".to_string()),
                Amount::from(1.),
            )]),
        )]));
        let update = ActionCacheUpdate::from(
            RuleName::from("test".to_string()),
            base_state_hash,
            StateHash::new(),
        );
        cache.apply_action_update(update).unwrap();
        assert_eq!(
            cache
                .action(&RuleName::from("test".to_string()), &base_state_hash)
                .unwrap(),
            StateHash::new()
        );
    }

    #[test]
    fn graph() {
        let mut cache = example_cache();
        let base_state = State::new();
        let base_state_hash = StateHash::from_state(&base_state);
        cache
            .add_condition(
                RuleName::from("test".to_string()),
                base_state_hash,
                RuleApplies::from(true),
            )
            .unwrap();
        cache
            .add_action(
                RuleName::from("test".to_string()),
                base_state_hash,
                StateHash::new(),
            )
            .unwrap();
        let mut possible_states = PossibleStates::new();
        possible_states
            .append_state(base_state_hash, base_state.clone())
            .unwrap();
        let graph = cache.graph(possible_states);
        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph[NodeIndex::from(0)], base_state);
    }

    #[test]
    fn merge() {
        let mut cache = example_cache();
        let base_state = State::from_entities(vec![(
            EntityName::from("test".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("test".to_string()),
                Amount::from(1.),
            )]),
        )]);
        let base_state_hash = StateHash::from_state(&base_state);
        cache
            .add_condition(
                RuleName::from("test".to_string()),
                base_state_hash,
                RuleApplies::from(true),
            )
            .unwrap();
        cache
            .add_action(
                RuleName::from("test".to_string()),
                base_state_hash,
                StateHash::new(),
            )
            .unwrap();
        let mut other_cache = example_cache();
        let other_base_state = State::from_entities(vec![(EntityName::new(), Entity::new())]);
        let other_base_state_hash = StateHash::from_state(&other_base_state);
        other_cache
            .add_condition(
                RuleName::from("test".to_string()),
                other_base_state_hash,
                RuleApplies::from(true),
            )
            .unwrap();
        other_cache
            .add_action(
                RuleName::from("test".to_string()),
                other_base_state_hash,
                StateHash::new(),
            )
            .unwrap();
        cache.merge(&other_cache).unwrap();
        assert!(cache
            .contains_condition(&RuleName::from("test".to_string()), &base_state_hash)
            .unwrap());
        assert!(cache
            .contains_condition(&RuleName::from("test".to_string()), &other_base_state_hash)
            .unwrap());
        assert_eq!(
            cache
                .action(&RuleName::from("test".to_string()), &base_state_hash)
                .unwrap(),
            StateHash::new()
        );
        assert_eq!(
            cache
                .action(&RuleName::from("test".to_string()), &other_base_state_hash)
                .unwrap(),
            StateHash::new()
        );
    }
}
