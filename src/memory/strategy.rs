use crate::types::Trajectory;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    pub name: String,
    pub weight: f64,
    pub success_count: u32,
    pub fail_count: u32,
}

#[derive(Debug, Default)]
pub struct StrategyMemory {
    strategies: HashMap<String, Strategy>,
}

impl StrategyMemory {
    pub fn absorb(&mut self, traj: &Trajectory) {
        let success = traj.prediction_errors.iter().all(|&e| e < 0.4);
        let key = if traj.actions.is_empty() {
            "noop".to_string()
        } else {
            traj.actions[0].name.clone()
        };
        self.bump(&key, success);
    }

    pub fn absorb_reward(&mut self, action: &str, reward: f64) {
        let success = reward >= 0.5;
        self.bump(action, success);
        let entry = self
            .strategies
            .entry(action.to_string())
            .or_insert(Strategy {
                name: action.to_string(),
                weight: 1.0,
                success_count: 0,
                fail_count: 0,
            });
        let delta = (reward - 0.5) * 0.2;
        entry.weight = (entry.weight + delta).clamp(0.1, 10.0);
    }

    fn bump(&mut self, key: &str, success: bool) {
        let entry = self.strategies.entry(key.to_string()).or_insert(Strategy {
            name: key.to_string(),
            weight: 1.0,
            success_count: 0,
            fail_count: 0,
        });
        if success {
            entry.success_count += 1;
            entry.weight = (entry.weight * 1.05).min(10.0);
        } else {
            entry.fail_count += 1;
            entry.weight = (entry.weight * 0.95).max(0.1);
        }
    }

    pub fn retrieve(&self, _query: &str, k: usize) -> Vec<Strategy> {
        let mut v: Vec<_> = self.strategies.values().cloned().collect();
        v.sort_by(|a, b| {
            b.weight
                .partial_cmp(&a.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        v.into_iter().take(k).collect()
    }

    pub fn snapshot(&self) -> HashMap<String, Strategy> {
        self.strategies.clone()
    }

    pub fn restore(strategies: HashMap<String, Strategy>) -> Self {
        Self { strategies }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Action, Trajectory};
    use std::collections::HashMap;

    fn traj_with_action(name: &str, low_error: bool) -> Trajectory {
        Trajectory {
            observations: vec![],
            predictions: vec![],
            actions: vec![Action {
                name: name.into(),
                params: HashMap::new(),
                constraints_checked: true,
            }],
            outcomes: vec![],
            prediction_errors: if low_error { vec![0.1] } else { vec![0.8] },
            meta: HashMap::new(),
        }
    }

    #[test]
    fn success_increases_weight() {
        let mut sm = StrategyMemory::default();
        sm.absorb(&traj_with_action("observe", true));
        sm.absorb(&traj_with_action("observe", true));
        let strategies = sm.retrieve("any", 5);
        assert_eq!(strategies.len(), 1);
        assert!(strategies[0].weight > 1.0);
        assert_eq!(strategies[0].success_count, 2);
    }

    #[test]
    fn failure_decreases_weight() {
        let mut sm = StrategyMemory::default();
        sm.absorb(&traj_with_action("risky", false));
        let strategies = sm.retrieve("any", 5);
        assert!(strategies[0].weight < 1.0);
        assert_eq!(strategies[0].fail_count, 1);
    }
}
