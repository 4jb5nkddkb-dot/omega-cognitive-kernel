//! Standard UCB decision maker for the nonstationary 2-armed bandit task.
//!
//! This is the "standard interpretation": maintain per-arm statistics and
//! pick the arm with the highest upper confidence bound. It coexists with
//! the existing ConstrainedDecisionMaker and is intended for bandit benchmarks
//! and similar discrete choice tasks.

use crate::error::Result;
use crate::l0::decision::DecisionMaker;
use crate::types::{Action, Intent, Prediction};
use hashbrown::HashMap;

#[derive(Debug, Clone)]
pub struct BanditUcbDecisionMaker {
    /// Empirical mean reward for arm 0 / arm 1.
    means: [f64; 2],
    /// Number of pulls for each arm.
    counts: [u64; 2],
    /// Total steps (for UCB exploration term).
    total: u64,
    /// Exploration coefficient (classic UCB1 uses sqrt(2)).
    c: f64,
}

impl Default for BanditUcbDecisionMaker {
    fn default() -> Self {
        Self {
            means: [0.5, 0.5],
            counts: [0, 0],
            total: 0,
            c: 1.41421356237, // sqrt(2)
        }
    }
}

impl BanditUcbDecisionMaker {
    pub fn new(c: f64) -> Self {
        Self {
            c: c.max(0.1),
            ..Self::default()
        }
    }

    /// Update statistics after observing a reward for an arm.
    /// Call this from the task loop after `record_reward`.
    pub fn observe(&mut self, arm: usize, reward: f64) {
        if arm > 1 {
            return;
        }
        let r = reward.clamp(0.0, 1.0);
        let n = self.counts[arm];
        self.means[arm] = if n == 0 {
            r
        } else {
            (self.means[arm] * n as f64 + r) / (n as f64 + 1.0)
        };
        self.counts[arm] += 1;
        self.total += 1;
    }

    fn ucb(&self, arm: usize) -> f64 {
        let n = self.counts[arm];
        if n == 0 {
            return f64::INFINITY; // force initial exploration
        }
        let mean = self.means[arm];
        let bonus = self.c * ((self.total.max(1) as f64).ln() / n as f64).sqrt();
        mean + bonus
    }

    fn best_arm(&self) -> usize {
        if self.ucb(1) > self.ucb(0) {
            1
        } else {
            0
        }
    }
}

impl DecisionMaker for BanditUcbDecisionMaker {
    fn decide(
        &self,
        intent: &Intent,
        _predictions: &[Prediction],
        _constraints: &HashMap<String, f64>,
        _memory_ctx: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<Action>> {
        let arm = self.best_arm();
        let name = if arm == 1 { "explore" } else { "stabilize" };

        let mut params = HashMap::new();
        params.insert("goal".into(), serde_json::Value::String(intent.goal.clone()));
        params.insert("arm".into(), serde_json::json!(arm));
        params.insert("ucb0".into(), serde_json::json!(self.ucb(0)));
        params.insert("ucb1".into(), serde_json::json!(self.ucb(1)));
        params.insert("mean0".into(), serde_json::json!(self.means[0]));
        params.insert("mean1".into(), serde_json::json!(self.means[1]));

        Ok(vec![Action {
            name: name.into(),
            params,
            constraints_checked: true,
        }])
    }
}
