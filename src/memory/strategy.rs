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

        let entry = self.strategies.entry(key.clone()).or_insert(Strategy {
            name: key,
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
}
