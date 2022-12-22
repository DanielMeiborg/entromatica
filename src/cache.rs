use std::fmt::Display;
use std::fmt::Formatter;

#[allow(unused_imports)]
use hashbrown::{HashMap, HashSet};
#[allow(unused_imports)]
use itertools::Itertools;
use petgraph::graph::NodeIndex;
use petgraph::Graph;

use crate::error::*;
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
                    Some(new_state_hash) => {
                        writeln!(f, "Rule applies for {base_state_hash} -> {new_state_hash}")?
                    }
                    None => writeln!(f, "Rule applies for {base_state_hash}")?,
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

    pub fn condition(&self, base_state_hash: &StateHash) -> Option<&RuleApplies> {
        self.condition.get(base_state_hash)
    }

    pub fn action(&self, base_state_hash: &StateHash) -> Option<&StateHash> {
        self.actions.get(base_state_hash)
    }

    pub fn add_condition(
        &mut self,
        base_state_hash: StateHash,
        applies: RuleApplies,
    ) -> Result<(), AlreadyExistsError<(StateHash, RuleApplies), RuleCache>> {
        if self.condition.contains_key(&base_state_hash) {
            return Err(AlreadyExistsError::new(
                (base_state_hash, applies),
                self.clone(),
            ));
        }
        self.condition.insert(base_state_hash, applies);
        Ok(())
    }

    pub fn add_action(
        &mut self,
        base_state_hash: StateHash,
        new_state_hash: StateHash,
    ) -> Result<(), AlreadyExistsError<(StateHash, StateHash), RuleCache>> {
        if self.actions.contains_key(&base_state_hash) {
            return Err(AlreadyExistsError::new(
                (base_state_hash, new_state_hash),
                self.clone(),
            ));
        }
        self.actions.insert(base_state_hash, new_state_hash);
        Ok(())
    }
}

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

    pub(self) fn rule(&self, rule_name: &RuleName) -> Option<&RuleCache> {
        self.rules.get(rule_name)
    }

    pub(self) fn rule_mut(&mut self, rule_name: &RuleName) -> Option<&mut RuleCache> {
        self.rules.get_mut(rule_name)
    }

    pub(self) fn add_rule(&mut self, rule_name: RuleName) -> Result<(), InternalErrorKind> {
        if self.rules.contains_key(&rule_name) {
            return Err(InternalErrorKind::RuleAlreadyExists(
                AlreadyExistsError::new(rule_name, self.clone()),
            ));
        }
        self.rules.insert(rule_name, RuleCache::new());
        Ok(())
    }

    pub fn condition(
        &self,
        rule_name: &RuleName,
        base_state_hash: &StateHash,
    ) -> Option<&RuleApplies> {
        self.rule(rule_name)?.condition(base_state_hash)
    }

    pub fn action(&self, rule_name: &RuleName, base_state_hash: &StateHash) -> Option<StateHash> {
        self.rule(rule_name)?.action(base_state_hash).copied()
    }

    pub fn add_action(
        &mut self,
        rule_name: RuleName,
        base_state_hash: StateHash,
        new_state_hash: StateHash,
    ) -> Result<(), InternalErrorKind> {
        match self.rule_mut(&rule_name) {
            Some(rule_cache) => rule_cache
                .add_action(base_state_hash, new_state_hash)
                .map_err(|_| {
                    InternalErrorKind::ActionAlreadyExists(AlreadyExistsError::new(
                        (base_state_hash, new_state_hash),
                        self.clone(),
                    ))
                }),
            None => {
                self.add_rule(rule_name.clone())?;
                let err = InternalErrorKind::RuleNotFound(NotFoundError::new(
                    rule_name.clone(),
                    self.clone(),
                ));
                let rule_cache = self.rule_mut(&rule_name).ok_or(err)?;
                rule_cache
                    .add_action(base_state_hash, new_state_hash)
                    .map_err(|_| {
                        InternalErrorKind::ActionAlreadyExists(AlreadyExistsError::new(
                            (base_state_hash, new_state_hash),
                            self.clone(),
                        ))
                    })
            }
        }
    }

    pub fn add_condition(
        &mut self,
        rule_name: RuleName,
        base_state_hash: StateHash,
        applies: RuleApplies,
    ) -> Result<(), InternalErrorKind> {
        match self.rule_mut(&rule_name) {
            Some(rule_cache) => rule_cache
                .add_condition(base_state_hash, applies)
                .map_err(|_| {
                    InternalErrorKind::ConditionAlreadyExists(AlreadyExistsError::new(
                        (base_state_hash, applies),
                        self.clone(),
                    ))
                }),
            None => {
                self.add_rule(rule_name.clone())?;
                let err = InternalErrorKind::RuleNotFound(NotFoundError::new(
                    rule_name.clone(),
                    self.clone(),
                ));
                let rule_cache = self.rule_mut(&rule_name).ok_or(err)?;
                rule_cache
                    .add_condition(base_state_hash, applies)
                    .map_err(|_| {
                        InternalErrorKind::ConditionAlreadyExists(AlreadyExistsError::new(
                            (base_state_hash, applies),
                            self.clone(),
                        ))
                    })
            }
        }
    }

    pub fn apply_condition_update(
        &mut self,
        update: ConditionCacheUpdate,
    ) -> Result<(), InternalErrorKind> {
        self.add_condition(update.rule_name, update.base_state_hash, update.applies)
    }

    pub fn apply_action_update(
        &mut self,
        update: ActionCacheUpdate,
    ) -> Result<(), InternalErrorKind> {
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
                if rule_cache.condition(state_hash).is_some() {
                    if let Some(new_state_hash) = rule_cache.action(state_hash) {
                        let new_state_node = nodes.get(new_state_hash).unwrap();
                        graph.add_edge(*state_node, *new_state_node, rule_name.clone());
                    }
                }
            }
        }
        graph
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
            cache.condition(&rule_name, &base_state_hash).cloned(),
            Some(applies)
        );
        assert_eq!(
            cache.action(&rule_name, &base_state_hash),
            Some(new_state_hash)
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
            cache.condition(&rule_name, &base_state_hash).cloned(),
            Some(applies)
        );
        assert_eq!(
            cache.action(&rule_name, &base_state_hash),
            Some(new_state_hash)
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
            cache.condition(&rule_name, &base_state_hash).cloned(),
            Some(applies)
        );
        assert_eq!(
            cache.action(&rule_name, &base_state_hash),
            Some(new_state_hash)
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
