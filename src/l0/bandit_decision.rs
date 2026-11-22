//! Standard UCB decision maker for the nonstationary 2-armed bandit task.
//!
//! This is the "standard interpretation": maintain per-arm statistics and
//! pick the arm with the highest upper confidence bound. It coexists with
//! the existing ConstrainedDecisionMaker and is intended for bandit benchmarks
//! and similar discrete choice tasks.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::error::Result;
use crate::l0::decision::DecisionMaker;
use crate::types::{
    Action, BanditConfig, DecisionFeedback, Intent, L2DecisionReason, L2DecisionRecord, Prediction,
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BanditUcbSnapshot {
    pub means: [f64; 2],
    pub counts: [u64; 2],
    pub total: u64,
    pub current_arm: Option<usize>,
    pub dwell_steps: u64,
    pub safety_override: bool,
    pub last_decision: Option<L2DecisionRecord>,
    pub config: BanditConfig,
    /// Optional nonstationary discount in (0, 1]; 1 retains classic UCB means.
    pub discount_factor: f64,
}

impl Default for BanditUcbSnapshot {
    fn default() -> Self {
        Self {
            means: [0.5, 0.5],
            counts: [0, 0],
            total: 0,
            current_arm: None,
            dwell_steps: 0,
            safety_override: false,
            last_decision: None,
            config: BanditConfig::default(),
            discount_factor: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BanditUcbDecisionMaker {
    state: BanditUcbSnapshot,
    exploration: f64,
}

impl Default for BanditUcbDecisionMaker {
    fn default() -> Self {
        Self {
            state: BanditUcbSnapshot::default(),
            exploration: core::f64::consts::SQRT_2,
        }
    }
}

impl BanditUcbDecisionMaker {
    pub fn new(c: f64) -> Self {
        Self {
            exploration: c.max(0.1),
            ..Self::default()
        }
    }

    pub fn snapshot(&self) -> BanditUcbSnapshot {
        self.state.clone()
    }

    pub fn restore(&mut self, snapshot: BanditUcbSnapshot) {
        self.state = snapshot;
    }

    pub fn mean_reward(&self, arm: usize) -> Option<f64> {
        self.state.means.get(arm).copied()
    }

    pub fn config(&self) -> &BanditConfig {
        &self.state.config
    }

    fn sanitized_config(config: BanditConfig) -> BanditConfig {
        BanditConfig {
            switch_margin: config.switch_margin.max(0.0),
            rollback_penalty: config.rollback_penalty.max(0.0),
            switch_cost: config.switch_cost.max(0.0),
            ..config
        }
    }

    pub fn observe(&mut self, arm: usize, reward: f64) {
        self.observe_with_feedback(arm, reward, DecisionFeedback::default());
    }

    pub fn observe_with_feedback(&mut self, arm: usize, reward: f64, feedback: DecisionFeedback) {
        if arm > 1 {
            return;
        }
        if self.state.current_arm.is_none() {
            self.state.current_arm = Some(arm);
            self.state.dwell_steps = 1;
        }
        let switch_cost = self
            .state
            .last_decision
            .as_ref()
            .filter(|d| d.switched && d.selected_arm == arm)
            .map(|d| d.switch_cost_applied)
            .unwrap_or(0.0);
        let rollback = u64::from(feedback.l1_rollback) + u64::from(feedback.safety_degradation);
        let adjusted = reward.clamp(0.0, 1.0)
            - 0.25 * feedback.error_delta
            - self.state.config.rollback_penalty * rollback as f64
            - switch_cost;
        let n = self.state.counts[arm];
        let discount = self.state.discount_factor.clamp(0.01, 1.0);
        self.state.means[arm] = if discount < 1.0 {
            discount * self.state.means[arm] + (1.0 - discount) * adjusted
        } else {
            (self.state.means[arm] * n as f64 + adjusted) / (n as f64 + 1.0)
        };
        self.state.counts[arm] = if discount < 1.0 {
            (self.state.counts[arm] + 1).min(32)
        } else {
            self.state.counts[arm] + 1
        };
        self.state.total += 1;
        if let Some(decision) = self.state.last_decision.as_mut() {
            if decision.selected_arm == arm {
                decision.adjusted_reward = Some(adjusted);
                decision.rollback_count = rollback;
            }
        } else {
            self.state.last_decision = Some(L2DecisionRecord {
                selected_arm: arm,
                selected_bias: if arm == 1 { "explore" } else { "stabilize" }.into(),
                candidate_arm: arm,
                mean_reward: self.state.means[arm],
                exploration_bonus: self.bonus(arm),
                ucb_score: self.record_ucb(arm),
                candidate_ucb_score: self.record_ucb(arm),
                current_ucb_score: self.record_ucb(arm),
                reason: L2DecisionReason::Exploit,
                dwell_steps: self.state.dwell_steps,
                dwell_suppressed: false,
                margin_suppressed: false,
                switched: false,
                safety_override: false,
                adjusted_reward: Some(adjusted),
                rollback_count: rollback,
                switch_cost_applied: 0.0,
            });
        }
    }

    fn bonus(&self, arm: usize) -> f64 {
        let n = self.state.counts[arm];
        self.exploration * libm::sqrt(libm::log(self.state.total.max(1) as f64) / n.max(1) as f64)
    }

    fn ucb(&self, arm: usize) -> f64 {
        if self.state.counts[arm] == 0 {
            f64::INFINITY
        } else {
            self.state.means[arm] + self.bonus(arm)
        }
    }

    fn record_ucb(&self, arm: usize) -> Option<f64> {
        let score = self.ucb(arm);
        score.is_finite().then_some(score)
    }

    fn decide_arm(&mut self) -> L2DecisionRecord {
        let candidate = if self.ucb(1) > self.ucb(0) { 1 } else { 0 };
        let incumbent = self.state.current_arm;
        let previous_dwell = self.state.dwell_steps;
        let mut selected = incumbent.unwrap_or(candidate);
        let mut reason = if incumbent.is_none() {
            L2DecisionReason::InitialExplore
        } else if candidate == selected {
            if self.bonus(candidate) > self.state.means[candidate] {
                L2DecisionReason::Explore
            } else {
                L2DecisionReason::Exploit
            }
        } else {
            L2DecisionReason::Switched
        };
        let mut dwell_suppressed = false;
        let mut margin_suppressed = false;
        let safety_override = self.state.safety_override;

        if safety_override && selected != 0 {
            selected = 0;
            reason = L2DecisionReason::SafetyOverride;
        } else if let Some(current) = incumbent {
            if candidate != current && !safety_override && self.state.counts[candidate] > 0 {
                if previous_dwell < self.state.config.min_dwell_steps {
                    dwell_suppressed = true;
                }
                if self.ucb(candidate) < self.ucb(current) + self.state.config.switch_margin {
                    margin_suppressed = true;
                }
            }
            if dwell_suppressed || margin_suppressed {
                selected = current;
                if dwell_suppressed {
                    reason = L2DecisionReason::DwellHold;
                } else {
                    reason = L2DecisionReason::MarginHold;
                }
            } else if candidate != current {
                selected = candidate;
            }
        }

        let switched = incumbent.map(|arm| arm != selected).unwrap_or(false);
        self.state.dwell_steps = if switched {
            1
        } else {
            previous_dwell.saturating_add(1)
        };
        self.state.current_arm = Some(selected);
        self.state.safety_override = false;
        let record = L2DecisionRecord {
            selected_arm: selected,
            selected_bias: if selected == 1 {
                "explore"
            } else {
                "stabilize"
            }
            .into(),
            candidate_arm: candidate,
            mean_reward: self.state.means[selected],
            exploration_bonus: self.bonus(selected),
            ucb_score: self.record_ucb(selected),
            candidate_ucb_score: self.record_ucb(candidate),
            current_ucb_score: incumbent.and_then(|arm| self.record_ucb(arm)),
            reason,
            dwell_steps: self.state.dwell_steps,
            dwell_suppressed,
            margin_suppressed,
            switched,
            safety_override,
            adjusted_reward: None,
            rollback_count: 0,
            switch_cost_applied: if switched {
                self.state.config.switch_cost
            } else {
                0.0
            },
        };
        self.state.last_decision = Some(record.clone());
        record
    }
}

impl DecisionMaker for BanditUcbDecisionMaker {
    fn decide(
        &mut self,
        intent: &Intent,
        _predictions: &[Prediction],
        _constraints: &HashMap<String, f64>,
        memory_ctx: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<Action>> {
        let record = self.decide_arm();
        let name = record.selected_bias.clone();
        let effect = memory_ctx
            .get("cognitive_effect")
            .cloned()
            .unwrap_or_default();
        let mut params = HashMap::new();
        params.insert(
            "goal".into(),
            serde_json::Value::String(intent.goal.clone()),
        );
        params.insert("arm".into(), serde_json::json!(record.selected_arm));
        params.insert(
            "l2_decision".into(),
            serde_json::to_value(&record)
                .map_err(|e| crate::error::OmegaError::Memory(e.to_string()))?,
        );
        params.insert("rule_effect".into(), effect);
        Ok(vec![Action {
            name,
            params,
            constraints_checked: true,
        }])
    }

    fn observe(&mut self, arm: usize, reward: f64) {
        BanditUcbDecisionMaker::observe(self, arm, reward);
    }

    fn observe_with_feedback(&mut self, arm: usize, reward: f64, feedback: DecisionFeedback) {
        BanditUcbDecisionMaker::observe_with_feedback(self, arm, reward, feedback);
    }

    fn set_bandit_config(&mut self, config: BanditConfig) {
        self.state.config = Self::sanitized_config(config);
    }

    fn l2_decision(&self) -> Option<L2DecisionRecord> {
        self.state.last_decision.clone()
    }

    fn restore_l2_decision(&mut self, decision: L2DecisionRecord) {
        self.state.last_decision = Some(decision);
    }

    fn request_safety_override(&mut self) {
        self.state.safety_override = true;
    }

    fn bandit_snapshot(&self) -> Option<BanditUcbSnapshot> {
        Some(self.snapshot())
    }

    fn restore_bandit_snapshot(&mut self, snapshot: BanditUcbSnapshot) {
        self.restore(snapshot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decision(dm: &mut BanditUcbDecisionMaker) -> L2DecisionRecord {
        dm.decide_arm()
    }

    fn configured() -> BanditUcbDecisionMaker {
        let mut dm = BanditUcbDecisionMaker::default();
        dm.set_bandit_config(BanditConfig {
            min_dwell_steps: 3,
            switch_margin: 0.75,
            rollback_penalty: 2.0,
            switch_cost: 0.0,
        });
        dm
    }

    #[test]
    fn dwell_and_margin_suppress_ucb_switch() {
        let mut dm = configured();
        decision(&mut dm);
        dm.observe(0, 1.0);
        decision(&mut dm);
        dm.observe(1, 0.5);
        dm.observe(0, 0.0);
        dm.restore(BanditUcbSnapshot {
            means: [0.0, 0.5],
            counts: [32, 128],
            total: 9,
            current_arm: Some(0),
            dwell_steps: 2,
            ..dm.snapshot()
        });

        let held = decision(&mut dm);
        assert_eq!(held.selected_arm, 0);
        assert!(held.dwell_suppressed);
        assert_eq!(held.reason, L2DecisionReason::DwellHold);

        let held = decision(&mut dm);
        assert_eq!(held.reason, L2DecisionReason::MarginHold);
        assert!(held.margin_suppressed);
        assert_eq!(held.selected_arm, 0);

        dm.set_bandit_config(BanditConfig {
            switch_margin: 0.0,
            ..dm.config().clone()
        });
        dm.observe(1, 1.0);
        dm.observe(1, 1.0);
        let switched = decision(&mut dm);
        assert_eq!(switched.selected_arm, 1);
        assert!(switched.switched);
        assert_eq!(switched.reason, L2DecisionReason::Switched);
    }

    #[test]
    fn unseen_arm_can_initialize_during_dwell() {
        let mut dm = configured();
        decision(&mut dm);
        dm.observe(0, 1.0);
        let second = decision(&mut dm);
        assert_eq!(second.selected_arm, 1);
        assert!(!second.dwell_suppressed);
    }

    #[test]
    fn rollback_penalty_changes_mean_and_next_arm() {
        let mut penalized = configured();
        let mut normal = configured();
        for dm in [&mut penalized, &mut normal] {
            dm.restore(BanditUcbSnapshot {
                means: [0.8, 0.8],
                counts: [1, 16],
                total: 2,
                current_arm: Some(0),
                dwell_steps: 3,
                last_decision: None,
                ..dm.snapshot()
            });
        }
        penalized.observe_with_feedback(
            0,
            0.8,
            DecisionFeedback {
                l1_rollback: true,
                error_delta: -0.4,
                ..DecisionFeedback::default()
            },
        );
        normal.observe_with_feedback(
            0,
            0.8,
            DecisionFeedback {
                error_delta: -0.4,
                ..DecisionFeedback::default()
            },
        );
        assert!(penalized.mean_reward(0).unwrap() < normal.mean_reward(0).unwrap());
        assert_eq!(penalized.l2_decision().unwrap().rollback_count, 1);
        assert!(penalized.l2_decision().unwrap().adjusted_reward.unwrap() < 0.8);
        for dm in [&mut penalized, &mut normal] {
            let mut state = dm.snapshot();
            state.total = 256;
            state.means[1] = 0.4;
            state.counts[1] = state.counts[0];
            dm.restore(state);
            dm.set_bandit_config(BanditConfig {
                switch_margin: 0.0,
                ..dm.config().clone()
            });
        }
        assert_eq!(decision(&mut penalized).selected_arm, 1);
        assert_eq!(decision(&mut normal).selected_arm, 0);
    }

    #[test]
    fn safety_override_bypasses_dwell() {
        let mut dm = configured();
        decision(&mut dm);
        dm.observe(0, 1.0);
        decision(&mut dm);
        dm.observe(1, 0.2);
        dm.request_safety_override();
        let decision = decision(&mut dm);
        assert_eq!(decision.reason, L2DecisionReason::SafetyOverride);
        assert_eq!(decision.selected_arm, 0);
        assert!(!decision.dwell_suppressed);
    }
}
