#[allow(unused_imports)]
use crate::prelude::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RewardSignal {
    pub raw_reward: f64,
    pub error_penalty: f64,
    pub progress_bonus: f64,
    pub net_signal: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Critic {
    pub error_weight: f64,
    pub progress_weight: f64,
}

impl Default for Critic {
    fn default() -> Self {
        Self {
            error_weight: 0.25,
            progress_weight: 0.40,
        }
    }
}

impl Critic {
    pub fn evaluate(
        &self,
        raw_reward: f64,
        prediction_error: f64,
        goal_progress: f64,
    ) -> RewardSignal {
        let error_penalty = self.error_weight * prediction_error.max(0.0);
        let progress_bonus = self.progress_weight * goal_progress;
        let net_signal = raw_reward - error_penalty + progress_bonus;
        RewardSignal {
            raw_reward,
            error_penalty,
            progress_bonus,
            net_signal,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ReflectionReport {
    pub total_cycles: u64,
    pub apply_count: u64,
    pub hold_count: u64,
    pub escalate_count: u64,
    pub mean_error: f64,
    pub success_rate: f64,
    pub stall_streak: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrialBudget {
    pub max_trials: u64,
    pub used_trials: u64,
    pub successes: u64,
}

impl TrialBudget {
    pub fn new(max_trials: u64) -> Self {
        Self {
            max_trials: max_trials.max(1),
            used_trials: 0,
            successes: 0,
        }
    }

    pub fn exhausted(&self) -> bool {
        self.used_trials >= self.max_trials
    }

    pub fn remaining(&self) -> u64 {
        self.max_trials.saturating_sub(self.used_trials)
    }

    pub fn record_trial(&mut self, success: bool) -> bool {
        if self.exhausted() {
            return false;
        }
        self.used_trials += 1;
        if success {
            self.successes += 1;
        }
        !self.exhausted()
    }

    pub fn reset(&mut self) {
        self.used_trials = 0;
        self.successes = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn critic_and_trial_budget_enforce_bounds() {
        let critic = Critic::default();
        let sig = critic.evaluate(1.0, 0.4, 0.5);
        assert!((sig.net_signal - 1.1).abs() < 1e-9);

        let mut budget = TrialBudget::new(2);
        assert!(budget.record_trial(false));
        assert_eq!(budget.remaining(), 1);
        assert!(!budget.record_trial(true));
        assert!(budget.exhausted());
        assert!(!budget.record_trial(true));
    }
}
