#[allow(unused_imports)]
use crate::prelude::*;

use alloc::collections::{BTreeMap, BTreeSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalStatus {
    Active,
    Achieved,
    Suspended,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalNode {
    pub id: String,
    pub targets: BTreeMap<String, f64>,
    pub weight: f64,
    pub priority: i32,
    pub status: GoalStatus,
    pub progress: f64,
    pub parent_id: Option<String>,
    pub children: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalTreeSummary {
    pub total_nodes: usize,
    pub achieved_nodes: usize,
    pub active_id: Option<String>,
    pub root_progress: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GoalTree {
    pub nodes: BTreeMap<String, GoalNode>,
    pub root_id: Option<String>,
    pub active_id: Option<String>,
}

impl GoalTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_flat(targets: BTreeMap<String, f64>) -> Self {
        let mut tree = Self::new();
        tree.add_root("root", targets, 1);
        tree
    }

    pub fn to_flat(&self) -> BTreeMap<String, f64> {
        self.active_id
            .as_ref()
            .and_then(|id| self.nodes.get(id))
            .map(|node| node.targets.clone())
            .or_else(|| {
                self.root_id
                    .as_ref()
                    .and_then(|id| self.nodes.get(id))
                    .map(|node| node.targets.clone())
            })
            .unwrap_or_default()
    }

    pub fn add_root(
        &mut self,
        id: impl Into<String>,
        targets: BTreeMap<String, f64>,
        priority: i32,
    ) {
        let id_str = id.into();
        let node = GoalNode {
            id: id_str.clone(),
            targets,
            weight: 1.0,
            priority,
            status: GoalStatus::Active,
            progress: 0.0,
            parent_id: None,
            children: Vec::new(),
        };
        self.nodes.insert(id_str.clone(), node);
        self.root_id = Some(id_str.clone());
        if self.active_id.is_none() {
            self.active_id = Some(id_str);
        }
    }

    pub fn add_child(
        &mut self,
        parent_id: &str,
        id: impl Into<String>,
        targets: BTreeMap<String, f64>,
        weight: f64,
        priority: i32,
    ) -> bool {
        if !self.nodes.contains_key(parent_id) {
            return false;
        }
        let child_id = id.into();
        if self.nodes.contains_key(&child_id) {
            return false;
        }
        let node = GoalNode {
            id: child_id.clone(),
            targets,
            weight: if weight.is_finite() && weight > 0.0 {
                weight
            } else {
                1.0
            },
            priority,
            status: GoalStatus::Active,
            progress: 0.0,
            parent_id: Some(parent_id.to_string()),
            children: Vec::new(),
        };
        self.nodes.insert(child_id.clone(), node);
        if let Some(parent) = self.nodes.get_mut(parent_id) {
            if !parent.children.iter().any(|c| c == &child_id) {
                parent.children.push(child_id);
            }
        }
        true
    }

    pub fn update_from_state(&mut self, state: &BTreeMap<String, f64>, tolerance: f64) {
        let tol = if tolerance.is_finite() && tolerance > 0.0 {
            tolerance
        } else {
            0.05
        };
        let leaf_ids: Vec<String> = self
            .nodes
            .iter()
            .filter(|(_, n)| n.children.is_empty())
            .map(|(id, _)| id.clone())
            .collect();

        for id in leaf_ids {
            if let Some(node) = self.nodes.get_mut(&id) {
                if node.targets.is_empty() {
                    continue;
                }
                let mut total_dist = 0.0;
                for (k, target) in &node.targets {
                    let cur = state.get(k).copied().unwrap_or(0.0);
                    total_dist += (cur - target).abs();
                }
                let mean_dist = total_dist / node.targets.len() as f64;
                let prog = (1.0 - mean_dist).clamp(0.0, 1.0);
                node.progress = prog;
                if mean_dist <= tol && node.status == GoalStatus::Active {
                    node.status = GoalStatus::Achieved;
                }
            }
        }

        if let Some(root) = self.root_id.clone() {
            self.rollup_node(&root);
        }
    }

    fn rollup_node(&mut self, id: &str) -> f64 {
        // Use an explicit post-order stack so a valid, deeply nested goal tree
        // cannot exhaust the call stack. The visited set also bounds traversal
        // if a deserialized or directly-mutated tree contains malformed links.
        let mut stack = vec![(id.to_string(), false)];
        let mut scheduled = BTreeSet::new();

        while let Some((node_id, expanded)) = stack.pop() {
            if expanded {
                let children = self
                    .nodes
                    .get(&node_id)
                    .map(|node| node.children.clone())
                    .unwrap_or_default();
                let mut weighted_sum = 0.0;
                let mut weight_total = 0.0;
                let mut all_achieved = true;
                for child_id in &children {
                    if let Some(child) = self.nodes.get(child_id) {
                        weighted_sum += child.progress * child.weight;
                        weight_total += child.weight;
                        if child.status != GoalStatus::Achieved {
                            all_achieved = false;
                        }
                    }
                }
                let progress = if weight_total > 0.0 {
                    (weighted_sum / weight_total).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                if let Some(node) = self.nodes.get_mut(&node_id) {
                    node.progress = progress;
                    if all_achieved && node.status == GoalStatus::Active {
                        node.status = GoalStatus::Achieved;
                    }
                }
                continue;
            }

            if !scheduled.insert(node_id.clone()) {
                continue;
            }
            let children = match self.nodes.get(&node_id) {
                Some(node) if !node.children.is_empty() => node.children.clone(),
                _ => continue,
            };
            stack.push((node_id, true));
            for child_id in children.into_iter().rev() {
                if !scheduled.contains(&child_id) {
                    stack.push((child_id, false));
                }
            }
        }

        self.nodes.get(id).map(|node| node.progress).unwrap_or(0.0)
    }

    pub fn select_active(&mut self) -> Option<&GoalNode> {
        let best_id = self
            .nodes
            .values()
            .filter(|n| n.status == GoalStatus::Active && n.children.is_empty())
            .max_by(|a, b| {
                a.priority
                    .cmp(&b.priority)
                    .then_with(|| b.progress.total_cmp(&a.progress))
            })
            .map(|n| n.id.clone());

        self.active_id = best_id;
        self.active_id.as_ref().and_then(|id| self.nodes.get(id))
    }

    pub fn summary(&self) -> GoalTreeSummary {
        let achieved_nodes = self
            .nodes
            .values()
            .filter(|n| n.status == GoalStatus::Achieved)
            .count();
        let root_progress = self
            .root_id
            .as_ref()
            .and_then(|id| self.nodes.get(id))
            .map(|n| n.progress)
            .unwrap_or(0.0);
        GoalTreeSummary {
            total_nodes: self.nodes.len(),
            achieved_nodes,
            active_id: self.active_id.clone(),
            root_progress,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_active_clears_active_id_when_all_leaves_are_achieved() {
        let mut tree = GoalTree::new();
        tree.add_root("root", BTreeMap::new(), 1);
        assert!(tree.add_child("root", "first", BTreeMap::new(), 1.0, 10));
        assert!(tree.add_child("root", "second", BTreeMap::new(), 1.0, 5));

        assert!(tree.select_active().is_some());
        assert!(tree.active_id.is_some());
        for node in tree
            .nodes
            .values_mut()
            .filter(|node| node.children.is_empty())
        {
            node.status = GoalStatus::Achieved;
        }

        assert!(tree.select_active().is_none());
        assert_eq!(tree.active_id, None);
    }

    #[test]
    fn hierarchical_rollup_and_active_selection() {
        let mut tree = GoalTree::new();
        tree.add_root("root", BTreeMap::new(), 1);

        let mut t1 = BTreeMap::new();
        t1.insert("x".into(), 1.0);
        assert!(tree.add_child("root", "sub1", t1, 1.0, 10));

        let mut t2 = BTreeMap::new();
        t2.insert("y".into(), 1.0);
        assert!(tree.add_child("root", "sub2", t2, 1.0, 5));

        let mut state = BTreeMap::new();
        state.insert("x".into(), 1.0);
        state.insert("y".into(), 0.0);
        tree.update_from_state(&state, 0.05);

        assert_eq!(tree.nodes["sub1"].status, GoalStatus::Achieved);
        assert_eq!(tree.nodes["sub2"].status, GoalStatus::Active);
        assert!((tree.summary().root_progress - 0.5).abs() < 1e-9);
        assert_eq!(tree.select_active().map(|n| n.id.as_str()), Some("sub2"));
    }

    #[test]
    fn add_child_rejects_duplicate_ids_and_cycles_without_mutating_tree() {
        let mut tree = GoalTree::new();
        tree.add_root("root", BTreeMap::new(), 1);
        assert!(tree.add_child("root", "branch", BTreeMap::new(), 1.0, 1));
        assert!(tree.add_child("branch", "leaf", BTreeMap::new(), 1.0, 1));

        let original = tree.clone();
        let state = BTreeMap::new();
        for (parent_id, duplicate_id) in [
            ("root", "leaf"),   // duplicate ID from a different branch depth
            ("leaf", "leaf"),   // self-cycle
            ("root", "root"),   // root as its own child
            ("leaf", "root"),   // root/ancestor cycle
            ("leaf", "branch"), // ancestor cycle
        ] {
            assert!(!tree.add_child(parent_id, duplicate_id, BTreeMap::new(), 1.0, 1,));
            assert_eq!(tree, original);
            tree.update_from_state(&state, 0.05);
            assert_eq!(tree, original);
        }
    }

    #[test]
    fn deep_goal_tree_rollup_does_not_use_call_stack() {
        let mut tree = GoalTree::new();
        tree.add_root("root", BTreeMap::new(), 1);
        let depth = 4_096;
        for index in 0..depth {
            let parent = if index == 0 {
                "root".to_string()
            } else {
                format!("node-{}", index - 1)
            };
            let targets = if index + 1 == depth {
                BTreeMap::from([("x".to_string(), 1.0)])
            } else {
                BTreeMap::new()
            };
            assert!(tree.add_child(&parent, format!("node-{index}"), targets, 1.0, 1));
        }

        let state = BTreeMap::from([("x".to_string(), 1.0)]);
        let result = std::thread::Builder::new()
            .stack_size(64 * 1024)
            .spawn(move || {
                tree.update_from_state(&state, 0.05);
                (tree.summary().root_progress, tree.nodes["root"].status)
            })
            .expect("spawn small-stack test thread")
            .join()
            .expect("deep tree rollup must not overflow the call stack");

        assert_eq!(result.0, 1.0);
        assert_eq!(result.1, GoalStatus::Achieved);
    }

    #[test]
    fn rollup_terminates_for_malformed_child_links() {
        let mut tree = GoalTree::new();
        tree.add_root("root", BTreeMap::new(), 1);
        assert!(tree.add_child("root", "child", BTreeMap::new(), 1.0, 1));
        tree.nodes
            .get_mut("child")
            .unwrap()
            .children
            .push("root".into());
        tree.nodes
            .get_mut("root")
            .unwrap()
            .children
            .push("missing".into());

        tree.update_from_state(&BTreeMap::new(), 0.05);

        assert_eq!(tree.nodes["root"].progress, 0.0);
        assert_eq!(tree.nodes["child"].progress, 0.0);
    }
}
