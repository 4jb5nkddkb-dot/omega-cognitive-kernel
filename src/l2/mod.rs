#[allow(unused_imports)]
use crate::prelude::*;

pub mod emergence;
pub mod fixed_point;
pub mod modulator;
pub mod monitor;

use crate::error::Result;
use crate::types::{MetaMetrics, RuleEffect, SystemState};
use emergence::EmergenceDetector;
use fixed_point::{OmegaDayanFixedPoint, ScoreSignals};
use hashbrown::HashMap;
use modulator::MetaModulator;
use monitor::MetaMonitor;

#[derive(Debug)]
pub struct L2Result {
    pub metrics: MetaMetrics,
    pub in_fixed_point: bool,
    pub biases: HashMap<String, f64>,
    pub constraints: HashMap<String, f64>,
    pub entered_fixed_point: bool,
    pub exited_fixed_point: bool,
    pub alignment_score: f64,
    pub candidate_alignment_score: f64,
    pub dwell_steps: u64,
    pub candidate_score: f64,
    pub modulation_proposed: bool,
    pub modulation_committed: bool,
    pub modulation_gain: f64,
    pub score_breakdown: HashMap<String, f64>,
    /// Soft-updated RuleEffect after L2 modulation (if any).
    pub effect_nudge: Option<RuleEffect>,
    /// The local UCB selection that caused this step's L1/L2 consequences.
    pub decision: Option<crate::types::L2DecisionRecord>,
    /// Latest candidate fixed-point residual: alignment target minus raw alignment.
    pub dayan_residual: f64,
}

pub struct L2MetaLayer {
    pub monitor: MetaMonitor,
    pub detector: EmergenceDetector,
    pub fixed_point: OmegaDayanFixedPoint,
    pub modulator: MetaModulator,
    /// Whether L2 is allowed to change biases, constraints, or RuleEffect.
    /// The monitor still runs when disabled so benchmark baselines retain
    /// comparable observability without receiving L2 interventions.
    ///
    /// Default is **false**. Enable explicitly with `Kernel::set_l2_enabled(true)`
    /// only after reproducible benchmarks show a positive gain over the disabled baseline.
    pub enabled: bool,
    pub in_fixed_point: bool,
    /// How many consecutive steps we have been inside the fixed point.
    pub dwell_steps: u64,
    pub last_alignment_score: f64,
    pub commit_threshold: f64,
    pub commit_cooldown_steps: u64,
    pub commit_cooldown_remaining: u64,
    pub last_consequence_score: f64,
    pub defensive_mode: bool,
    pub negative_consequence_streak: u32,
    pub rollback_effect: Option<RuleEffect>,
    pub external_attention: bool,
    pub stable_consequence_steps: u32,
    pub reward_change_score: f64,
    pub error_change_score: f64,
    pub awaiting_consequence: bool,
    pub negative_reward_streak: u32,
    pub negative_reward_required: u32,
}

impl L2MetaLayer {
    pub fn new() -> Self {
        Self {
            monitor: MetaMonitor,
            detector: EmergenceDetector::default(),
            fixed_point: OmegaDayanFixedPoint,
            modulator: MetaModulator,
            enabled: false,
            in_fixed_point: false,
            dwell_steps: 0,
            last_alignment_score: 0.0,
            commit_threshold: 0.82,
            commit_cooldown_steps: 5,
            commit_cooldown_remaining: 0,
            last_consequence_score: 0.0,
            defensive_mode: false,
            negative_consequence_streak: 0,
            rollback_effect: None,
            external_attention: false,
            stable_consequence_steps: 0,
            reward_change_score: 0.0,
            error_change_score: 0.0,
            awaiting_consequence: false,
            negative_reward_streak: 0,
            negative_reward_required: 3,
        }
    }

    pub fn step(
        &mut self,
        state: &SystemState,
        biases: HashMap<String, f64>,
        constraints: HashMap<String, f64>,
        current_effect: &RuleEffect,
    ) -> Result<L2Result> {
        let metrics = self.monitor.evaluate(state)?;

        let mut new_biases = biases;
        let mut new_constraints = constraints;
        let mut entered = false;
        let mut exited = false;
        let mut alignment_score = self.last_alignment_score;
        let mut candidate_alignment_score = 0.0;
        let mut effect_nudge = None;
        let mut candidate_score = 0.0;
        let mut modulation_proposed = false;
        let mut modulation_committed = false;
        let mut modulation_gain = 0.0;
        let mut score_breakdown = HashMap::new();

        self.commit_cooldown_remaining = self.commit_cooldown_remaining.saturating_sub(1);
        let attention = self.external_attention;
        self.external_attention = false;

        if !self.enabled {
            self.in_fixed_point = false;
            self.dwell_steps = 0;
            return Ok(L2Result {
                metrics,
                in_fixed_point: false,
                biases: new_biases,
                constraints: new_constraints,
                entered_fixed_point: false,
                exited_fixed_point: false,
                alignment_score: 0.0,
                candidate_alignment_score,
                dwell_steps: 0,
                candidate_score,
                modulation_proposed,
                modulation_committed,
                modulation_gain,
                score_breakdown,
                effect_nudge: None,
                decision: None,
                dayan_residual: 1.0,
            });
        }

        if self.in_fixed_point {
            self.dwell_steps = self.dwell_steps.saturating_add(1);
            if self.detector.should_exit(&metrics) || self.dwell_steps > 40 {
                self.in_fixed_point = false;
                self.dwell_steps = 0;
                exited = true;
            } else if self.dwell_steps % 6 == 0 {
                let fp = self.fixed_point.enter_with_signals(
                    state,
                    ScoreSignals {
                        reward_change_score: self.reward_change_score,
                        error_change_score: self.error_change_score,
                        stability_score: metrics.self_ref_stability,
                    },
                )?;
                alignment_score = fp.score;
                candidate_alignment_score = fp.alignment_score;
                self.last_alignment_score = fp.score;
                candidate_score = fp.score;
                score_breakdown = fp.breakdown.clone();
                modulation_proposed = true;
                if fp.score >= self.commit_threshold
                    && self.commit_cooldown_remaining == 0
                    && !self.defensive_mode
                    && (self.stable_consequence_steps < 8 || attention)
                {
                    modulation_committed = true;
                    modulation_gain = ((fp.score - self.commit_threshold)
                        / (1.0 - self.commit_threshold))
                        .clamp(0.10, 1.0);
                    let (b, c) = self.modulator.apply_scaled(
                        &fp,
                        modulation_gain,
                        new_biases,
                        new_constraints,
                    );
                    new_biases = b;
                    new_constraints = c;
                    effect_nudge = Some(self.modulator.apply_to_effect_scaled(
                        &fp,
                        modulation_gain,
                        current_effect,
                    ));
                    self.commit_cooldown_remaining = self.commit_cooldown_steps;
                    self.awaiting_consequence = true;
                }
            }
        } else if self.detector.should_trigger(&metrics) || attention {
            let fp = self.fixed_point.enter_with_signals(
                state,
                ScoreSignals {
                    reward_change_score: self.reward_change_score,
                    error_change_score: self.error_change_score,
                    stability_score: metrics.self_ref_stability,
                },
            )?;
            candidate_score = fp.score;
            score_breakdown = fp.breakdown.clone();
            modulation_proposed = true;
            alignment_score = fp.score;
            candidate_alignment_score = fp.alignment_score;
            self.last_alignment_score = fp.score;
            entered = fp.aligned;
            self.in_fixed_point = fp.aligned;
            if fp.aligned {
                self.dwell_steps = 1;
            }
            if fp.score >= self.commit_threshold
                && self.commit_cooldown_remaining == 0
                && !self.defensive_mode
                && (self.stable_consequence_steps < 8 || attention)
            {
                modulation_committed = true;
                modulation_gain = ((fp.score - self.commit_threshold)
                    / (1.0 - self.commit_threshold))
                    .clamp(0.10, 1.0);
                let (b, c) =
                    self.modulator
                        .apply_scaled(&fp, modulation_gain, new_biases, new_constraints);
                new_biases = b;
                new_constraints = c;
                effect_nudge = Some(self.modulator.apply_to_effect_scaled(
                    &fp,
                    modulation_gain,
                    current_effect,
                ));
                self.rollback_effect = Some(current_effect.clone());
                self.commit_cooldown_remaining = self.commit_cooldown_steps;
                self.awaiting_consequence = true;
            }
        }

        Ok(L2Result {
            metrics,
            in_fixed_point: self.in_fixed_point,
            biases: new_biases,
            constraints: new_constraints,
            entered_fixed_point: entered,
            exited_fixed_point: exited,
            alignment_score,
            candidate_alignment_score,
            dwell_steps: self.dwell_steps,
            candidate_score,
            modulation_proposed,
            modulation_committed,
            modulation_gain,
            score_breakdown,
            effect_nudge,
            decision: None,
            dayan_residual: (1.0 - candidate_alignment_score).abs(),
        })
    }

    /// Feed back the short-term consequence of the last committed modulation.
    /// Negative outcomes extend the commit cooldown and leave fixed-point mode.
    pub fn observe_consequence(&mut self, reward_delta: f64, error_delta: f64, risk_delta: f64) {
        let score = 0.60 * reward_delta - 0.25 * error_delta - 0.15 * risk_delta;
        self.last_consequence_score = score;
        if reward_delta < -0.05 {
            self.negative_reward_streak = self.negative_reward_streak.saturating_add(1);
        } else {
            self.negative_reward_streak = 0;
        }
        self.reward_change_score = if self.negative_reward_streak >= self.negative_reward_required {
            ((-reward_delta) / 0.5).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.error_change_score = (error_delta / 0.15).clamp(0.0, 1.0);
        if !self.awaiting_consequence {
            return;
        }
        self.awaiting_consequence = false;
        if score < -0.05 {
            self.external_attention = true;
            self.stable_consequence_steps = 0;
            self.negative_consequence_streak = self.negative_consequence_streak.saturating_add(1);
            self.commit_cooldown_remaining = self
                .commit_cooldown_remaining
                .max(self.commit_cooldown_steps.saturating_mul(2));
            self.in_fixed_point = false;
            self.dwell_steps = 0;
            if self.negative_consequence_streak >= 2 {
                self.defensive_mode = true;
            }
        } else if score > 0.05 {
            self.stable_consequence_steps = self.stable_consequence_steps.saturating_add(1);
            self.negative_consequence_streak = 0;
            if self.defensive_mode && score > 0.15 {
                self.defensive_mode = false;
            }
        }
    }

    pub fn take_rollback_effect(&mut self) -> Option<RuleEffect> {
        if self.last_consequence_score < -0.05 {
            self.rollback_effect.take()
        } else {
            None
        }
    }
}

impl Default for L2MetaLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::L2MetaLayer;
    use crate::types::{RuleEffect, SystemState};
    use hashbrown::HashMap;

    #[test]
    fn defensive_mode_recovers_after_strong_positive_consequence() {
        let mut l2 = L2MetaLayer::new();
        l2.enabled = true; // test requires L2 active for consequence path
        l2.defensive_mode = true;
        l2.negative_consequence_streak = 2;
        l2.awaiting_consequence = true;

        // Positive reward and slightly improving error produce a consequence
        // score above the recovery threshold while representing a real
        // post-commit observation.
        l2.observe_consequence(0.5, -0.01, 0.0);

        assert!(l2.last_consequence_score > 0.15);
        assert!(!l2.defensive_mode);
        assert_eq!(l2.negative_consequence_streak, 0);
        assert!(!l2.awaiting_consequence);
    }

    #[test]
    fn aligned_but_below_commit_threshold_does_not_nudge() {
        let mut l2 = L2MetaLayer::new();
        l2.enabled = true; // test requires L2 active
        l2.detector.window = 2;
        l2.detector.pred_threshold = 0.2;
        l2.detector.self_ref_threshold = 0.2;
        l2.detector.cooldown_steps = 0;
        l2.commit_threshold = 0.90;

        let mut active_code = HashMap::new();
        active_code.insert(
            "cognitive_rules".to_string(),
            "self-model bias damp rule".to_string(),
        );
        let state = SystemState {
            active_code,
            highest_attractor: "maintain coherent self-model omega dayan".to_string(),
            generative_base: Some("dayan_generative_base".to_string()),
            last_trajectory_summary: Some("coherent fixed-point candidate".to_string()),
            avg_prediction_error: 0.2,
            total_steps: 20,
        };

        let mut entered = None;
        for _ in 0..3 {
            let result = l2
                .step(
                    &state,
                    HashMap::new(),
                    HashMap::new(),
                    &RuleEffect::default(),
                )
                .expect("L2 step");
            if result.entered_fixed_point {
                entered = Some(result);
                break;
            }
        }

        let result = entered.expect("expected raw alignment to enter fixed-point");
        assert!(result.candidate_alignment_score > 0.82);
        assert!(result.candidate_score < l2.commit_threshold);
        assert!(result.in_fixed_point);
        assert!(result.dwell_steps >= 1);
        assert!(!result.modulation_committed);
        assert!(result.effect_nudge.is_none());
    }
}
