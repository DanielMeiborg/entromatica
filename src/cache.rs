#[allow(unused_imports)]
use hashbrown::{HashMap, HashSet};
#[allow(unused_imports)]
use itertools::Itertools;
use petgraph::graph::NodeIndex;
use petgraph::Graph;

use crate::rules::*;
use crate::state::*;

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub(self) struct RuleCache {
    condition: HashMap<StateHash, RuleApplies>,
    actions: HashMap<StateHash, StateHash>,
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
    ) -> Result<(), String> {
        if self.condition.contains_key(&base_state_hash) {
            return Err("Condition already exists in cache".to_string());
        }
        self.condition.insert(base_state_hash, applies);
        Ok(())
    }

    pub fn add_action(
        &mut self,
        base_state_hash: StateHash,
        new_state_hash: StateHash,
    ) -> Result<(), String> {
        if self.actions.contains_key(&base_state_hash) {
            return Err("Action already exists in cache".to_string());
        }
        self.actions.insert(base_state_hash, new_state_hash);
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub(crate) struct Cache {
    rules: HashMap<RuleName, RuleCache>,
}

impl Cache {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    pub(self) fn rule(&self, rule_name: &RuleName) -> Option<RuleCache> {
        self.rules.get(rule_name).cloned()
    }

    pub(self) fn rule_mut(&mut self, rule_name: &RuleName) -> Option<&mut RuleCache> {
        self.rules.get_mut(rule_name)
    }

    pub(self) fn add_rule(&mut self, rule_name: RuleName) -> Result<(), String> {
        if self.rules.contains_key(&rule_name) {
            return Err("Rule already exists in cache".to_string());
        }
        self.rules.insert(rule_name, RuleCache::new());
        Ok(())
    }

    pub fn condition(
        &self,
        rule_name: &RuleName,
        base_state_hash: &StateHash,
    ) -> Option<RuleApplies> {
        self.rule(rule_name)?.condition(base_state_hash).copied()
    }

    pub fn action(&self, rule_name: &RuleName, base_state_hash: &StateHash) -> Option<StateHash> {
        self.rule(rule_name)?.action(base_state_hash).copied()
    }

    pub fn add_action(
        &mut self,
        rule_name: RuleName,
        base_state_hash: StateHash,
        new_state_hash: StateHash,
    ) -> Result<(), String> {
        match self.rule_mut(&rule_name) {
            Some(rule_cache) => rule_cache.add_action(base_state_hash, new_state_hash),
            None => {
                self.add_rule(rule_name.clone())?;
                let rule_cache = self.rule_mut(&rule_name).unwrap();
                rule_cache.add_action(base_state_hash, new_state_hash)
            }
        }
    }

    pub fn add_condition(
        &mut self,
        rule_name: RuleName,
        base_state_hash: StateHash,
        applies: RuleApplies,
    ) -> Result<(), String> {
        match self.rule_mut(&rule_name) {
            Some(rule_cache) => rule_cache.add_condition(base_state_hash, applies),
            None => {
                self.add_rule(rule_name.clone())?;
                let rule_cache = self.rule_mut(&rule_name).unwrap();
                rule_cache.add_condition(base_state_hash, applies)
            }
        }
    }

    pub fn apply_condition_update(&mut self, update: ConditionCacheUpdate) -> Result<(), String> {
        self.add_condition(update.rule_name, update.base_state_hash, update.applies)
    }

    pub fn apply_action_update(&mut self, update: ActionCacheUpdate) -> Result<(), String> {
        self.add_action(
            update.rule_name,
            update.base_state_hash,
            update.new_state_hash,
        )
    }

    ///Gets a graph from the possible states with the nodes being the states and the directed edges being the rule names.
    pub fn get_graph_from_cache(&self, possible_states: PossibleStates) -> Graph<State, RuleName> {
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
