use std::fmt::Display;
use std::fmt::Formatter;
use std::sync::mpsc::SendError;

#[allow(unused_imports)]
use hashbrown::{HashMap, HashSet};
#[allow(unused_imports)]
use itertools::Itertools;

use backtrace::Backtrace as trc;
use petgraph::graph::NodeIndex;
use petgraph::Graph;
use thiserror::Error;

use crate::rules::*;
use crate::state::*;

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
            .ok_or(RuleCacheError::ConditionNotFound {
                base_state_hash: *base_state_hash,
                context: trc::new(),
            })
    }

    pub fn action(&self, base_state_hash: &StateHash) -> Result<&StateHash, RuleCacheError> {
        self.actions
            .get(base_state_hash)
            .ok_or(RuleCacheError::ActionNotFound {
                base_state_hash: *base_state_hash,
                context: trc::new(),
            })
    }

    pub fn add_condition(
        &mut self,
        base_state_hash: StateHash,
        applies: RuleApplies,
    ) -> Result<(), RuleCacheError> {
        if self.condition.contains_key(&base_state_hash) {
            return Err(RuleCacheError::ConditionAlreadyExists {
                base_state_hash,
                applies,
                context: trc::new(),
            });
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
            return Err(RuleCacheError::ActionAlreadyExists {
                base_state_hash,
                new_state_hash,
                context: trc::new(),
            });
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
                context: trc::new(),
            })
    }

    pub(self) fn rule_mut(&mut self, rule_name: &RuleName) -> Result<&mut RuleCache, CacheError> {
        self.rules
            .get_mut(rule_name)
            .ok_or_else(|| CacheError::RuleNotFound {
                rule_name: rule_name.clone(),
                context: trc::new(),
            })
    }

    pub(self) fn add_rule(&mut self, rule_name: RuleName) -> Result<(), CacheError> {
        if self.rules.contains_key(&rule_name) {
            return Err(CacheError::RuleAlreadyExists {
                rule_name,
                context: trc::new(),
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
}

#[non_exhaustive]
#[derive(Debug, Clone, Error)]
pub(crate) enum CacheError {
    #[error("Rule already exists: {rule_name:#?}")]
    RuleAlreadyExists { rule_name: RuleName, context: trc },

    #[error("Rule not found: {rule_name:#?}")]
    RuleNotFound { rule_name: RuleName, context: trc },

    #[error("Condition cache update send error: {source:#?}")]
    ConditionCacheUpdateSendError {
        #[source]
        source: SendError<ConditionCacheUpdate>,
        context: trc,
    },

    #[error("Action cache update send error: {source:#?}")]
    ActionCacheUpdateSendError {
        #[source]
        source: SendError<ActionCacheUpdate>,
        context: trc,
    },

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
            context: trc::new(),
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
mod tests {
    use super::*;
    use crate::resource::*;
    use crate::units::*;

    #[test]
    fn cache_add_should_work() {
        let mut cache = Cache::new();
        let rule_name = RuleName::from("test".to_string());
        let base_state_hash = StateHash::new();
        let new_state_hash = StateHash::new();
        let applies = RuleApplies::from(true);
        cache
            .add_condition(rule_name.clone(), base_state_hash, applies)
            .unwrap();
        cache
            .add_action(rule_name.clone(), base_state_hash, new_state_hash)
            .unwrap();
        assert_eq!(
            cache
                .condition(&rule_name, &base_state_hash)
                .cloned()
                .unwrap(),
            applies
        );
        assert_eq!(
            cache.action(&rule_name, &base_state_hash).unwrap(),
            new_state_hash
        );
    }

    #[test]
    fn cache_no_overwriting_values() {
        let mut cache = Cache::new();
        let rule_name = RuleName::from("test".to_string());
        let base_state_hash = StateHash::new();
        let new_state_hash = StateHash::new();
        let applies = RuleApplies::from(true);
        cache
            .add_condition(rule_name.clone(), base_state_hash, applies)
            .unwrap();
        cache
            .add_action(rule_name.clone(), base_state_hash, new_state_hash)
            .unwrap();
        let new_new_state_hash = StateHash::from_state(&State::from_entities(vec![(
            EntityName::from("A".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("Resource".to_string()),
                Amount::from(0.),
            )]),
        )]));
        let new_applies = RuleApplies::from(false);
        cache
            .add_condition(rule_name.clone(), base_state_hash, new_applies)
            .unwrap_err();
        cache
            .add_action(rule_name.clone(), base_state_hash, new_new_state_hash)
            .unwrap_err();
        assert_eq!(
            cache
                .condition(&rule_name, &base_state_hash)
                .cloned()
                .unwrap(),
            applies
        );
        assert_eq!(
            cache.action(&rule_name, &base_state_hash).unwrap(),
            new_state_hash
        );
    }

    #[test]
    fn cache_apply_updates() {
        let mut cache = Cache::new();
        let rule_name = RuleName::from("test".to_string());
        let base_state_hash = StateHash::new();
        let new_state_hash = StateHash::new();
        let applies = RuleApplies::from(true);
        let condition_update =
            ConditionCacheUpdate::from(rule_name.clone(), base_state_hash, applies);
        let action_update =
            ActionCacheUpdate::from(rule_name.clone(), base_state_hash, new_state_hash);
        cache.apply_condition_update(condition_update).unwrap();
        cache.apply_action_update(action_update).unwrap();
        assert_eq!(
            cache
                .condition(&rule_name, &base_state_hash)
                .cloned()
                .unwrap(),
            applies
        );
        assert_eq!(
            cache.action(&rule_name, &base_state_hash).unwrap(),
            new_state_hash
        );
    }

    #[test]
    fn cache_get_graph() {
        let mut cache = Cache::new();
        let rule_name = RuleName::from("test".to_string());
        let base_state = State::new();
        let base_state_hash = StateHash::from_state(&base_state);
        let new_state = State::from_entities(vec![(
            EntityName::from("A".to_string()),
            Entity::from_resources(vec![(
                ResourceName::from("Resource".to_string()),
                Amount::from(0.),
            )]),
        )]);
        let new_state_hash = StateHash::from_state(&new_state);
        let applies = RuleApplies::from(true);
        let possible_states = PossibleStates::from(HashMap::from([
            (base_state_hash, base_state.clone()),
            (new_state_hash, new_state.clone()),
        ]));
        cache
            .add_condition(rule_name.clone(), base_state_hash, applies)
            .unwrap();
        cache
            .add_action(rule_name, base_state_hash, new_state_hash)
            .unwrap();

        let graph = cache.graph(possible_states);
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        assert_eq!(graph.raw_nodes()[0].weight, base_state);
        assert_eq!(graph.raw_nodes()[1].weight, new_state);
    }
}
