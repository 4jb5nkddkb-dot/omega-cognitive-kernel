#[allow(unused_imports)]
use crate::prelude::*;

pub mod control;
pub mod emergence;
pub mod fixed_point;
pub mod meta_cache;
pub mod modulator;
pub mod monitor;
pub mod spectral_gate;

pub use spectral_gate::{
    CognitiveRealm, KernelStepOutput, L2CognitiveGate, L2Config as SpectralGateConfig, L2Decision,
    L2State,
};

use crate::error::Result;
use crate::types::{MetaMetrics, RuleEffect, SystemState};
use control::{L2GateState, ResidentController, KEEP_L0};
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
    pub gate_state: L2GateState,
    pub alpha: f64,
    pub beta: f64,
    pub expert_weights: HashMap<String, f64>,
    pub lyapunov_potential: f64,
    pub lyapunov_veto: bool,
    pub meta_cache_hit: bool,
    pub selected_expert: String,
    pub l2_policy: crate::types::L2Policy,
    /// Policy that governed the L0 action in this result, not the policy queued
    /// for the next action. This distinction makes IPS and replay causal.
    pub applied_policy: crate::types::L2Policy,
    pub veto_reason: Option<String>,
    pub rejection_reason: Option<String>,
    pub spectral_output: Option<KernelStepOutput>,
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
    pub controller: ResidentController,
    pub pending_l1_rollback: bool,
    pub last_rejection_reason: Option<String>,
    pub pending_policy: crate::types::L2Policy,
    /// Validated policy used by the current L0 decision and its outcome.
    pub applied_policy: crate::types::L2Policy,
    pub last_error: Option<f64>,
    pub last_reward: f64,
    pub consecutive_l1_rollbacks: u32,
    pub current_cycle_recoveries: u32,
    pub spectral_reflector: crate::l1::spectral::L1SpectralReflector,
    pub spectral_gate: L2CognitiveGate,
    pub spectral_history: Vec<f64>,
}

impl L2MetaLayer {
    pub fn new() -> Self {
        Self {
            monitor: MetaMonitor,
            detector: EmergenceDetector::default(),
            fixed_point: OmegaDayanFixedPoint,
            modulator: MetaModulator,
            enabled: true,
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
            controller: ResidentController::default(),
            pending_l1_rollback: false,
            pending_policy: crate::types::L2Policy::default(),
            last_rejection_reason: None,
            applied_policy: crate::types::L2Policy::default(),
            last_error: None,
            last_reward: 0.5,
            consecutive_l1_rollbacks: 0,
            current_cycle_recoveries: 0,
            spectral_reflector: crate::l1::spectral::L1SpectralReflector::default(),
            spectral_gate: L2CognitiveGate::new(SpectralGateConfig::with_window(16)),
            spectral_history: Vec::new(),
        }
    }

    pub fn step(
        &mut self,
        state: &SystemState,
        biases: HashMap<String, f64>,
        constraints: HashMap<String, f64>,
        _current_effect: &RuleEffect,
    ) -> Result<L2Result> {
        let metrics = self.monitor.evaluate(state)?;
        let mut entered = false;
        let mut exited = false;
        if self.pending_l1_rollback {
            self.consecutive_l1_rollbacks = self.consecutive_l1_rollbacks.saturating_add(1);
            self.pending_l1_rollback = false;
        } else {
            self.consecutive_l1_rollbacks = 0;
        }
        self.commit_cooldown_remaining = self.commit_cooldown_remaining.saturating_sub(1);
        self.controller.meta_cache_hit = false;
        self.controller.certificate.lyapunov_veto = false;
        let mut modulation_proposed = false;
        let mut modulation_committed = false;
        let mut modulation_gain = 0.0;
        let mut rejection_reason = None;

        // Refresh observability even when the intervention switch is off. A
        // disabled baseline therefore reports the real residual, not a sentinel.
        let fp = self.fixed_point.enter_with_signals(
            state,
            ScoreSignals {
                reward_change_score: self.reward_change_score,
                error_change_score: self.error_change_score,
                stability_score: metrics.self_ref_stability,
            },
        )?;
        let candidate_alignment = fp.alignment_score;
        let candidate_score = fp.score;
        let score_breakdown = fp.breakdown.clone();
        self.last_alignment_score = fp.score;
        self.controller.update_dayan(candidate_alignment);

        if !self.enabled {
            self.in_fixed_point = false;
            self.dwell_steps = 0;
            self.controller.gate_state = L2GateState::Dormant;
            self.controller.retire_requested = false;
            self.controller.trust.alpha = 0.0;
            self.controller.last_selected_expert = KEEP_L0;
            self.pending_policy = crate::types::L2Policy::default();
            return self.result(
                metrics,
                biases,
                constraints,
                candidate_score,
                candidate_alignment,
                score_breakdown,
                false,
                false,
                false,
                0,
                candidate_score,
                modulation_proposed,
                modulation_committed,
                modulation_gain,
            );
        }

        if self.consecutive_l1_rollbacks >= self.controller.rollback_wakeup {
            self.controller.trust.alpha = 0.0;
            self.controller.trust.cooldown_remaining = self.controller.trust.cooldown_steps;
        }
        if self.in_fixed_point {
            self.dwell_steps = self.dwell_steps.saturating_add(1);
            if self.detector.should_exit(&metrics) || self.dwell_steps > 40 {
                self.in_fixed_point = false;
                self.dwell_steps = 0;
                exited = true;
            }
        } else if self.detector.should_trigger(&metrics) || self.external_attention {
            entered = fp.aligned;
            self.in_fixed_point = fp.aligned;
            if fp.aligned {
                self.dwell_steps = 1;
            }
        }
        self.external_attention = false;

        self.pending_policy = crate::types::L2Policy::default();
        if self.controller.gate_state == L2GateState::Active && self.controller.surprise_started {
            // Recording a recovery context is independent of proposal trust.
            // A cooldown suppresses intervention, not the episode's cache key.
            self.controller.cache_prepared = true;
            self.controller.cache_prepared_fingerprint = self.controller.surprise_fingerprint;
            self.controller.episode_start_residual = self.controller.residual;
            if let Some((_index, weights, alpha)) =
                self.controller.cache.lookup(&self.controller.fingerprint())
            {
                self.controller.experts.warm_start(weights);
                self.controller.trust.alpha = alpha.min(self.controller.trust.alpha_max);
                self.controller.meta_cache_hit = true;
                self.controller.cache_hits = self.controller.cache_hits.saturating_add(1);
            }
        }
        let can_propose = self.controller.gate_state == L2GateState::Active
            && self.controller.trust.cooldown_remaining == 0
            && !self.controller.retire_requested;
        if can_propose {
            let probabilities = self.controller.experts.probabilities();
            let selected = if self.controller.meta_cache_hit {
                (1..4)
                    .max_by(|a, b| {
                        self.controller.experts.weights[*a]
                            .partial_cmp(&self.controller.experts.weights[*b])
                            .unwrap_or(core::cmp::Ordering::Equal)
                    })
                    .unwrap_or(1)
            } else {
                self.controller.experts.select(self.controller.ema_error)
            };
            self.controller.last_selected_expert = selected;
            modulation_proposed = selected != KEEP_L0;
            if selected != KEEP_L0 {
                let predicted_residual = self.controller.predicted_residual(
                    state.avg_prediction_error,
                    self.controller.trust.alpha,
                    selected,
                );
                self.controller.certificate = self
                    .controller
                    .certificate
                    .check(self.controller.residual, predicted_residual);
                if self.controller.certificate.lyapunov_veto {
                    self.controller.veto_count = self.controller.veto_count.saturating_add(1);
                }
                let beta = self.controller.beta();
                // A high anomaly score is evidence for waking up, not evidence
                // that an intervention will help. Commit confidence comes from
                // the residual model; the Lyapunov certificate is the final
                // predicted-stability guard. This preserves the public
                // commit_threshold knob as a conservative confidence gate.
                let allowed = self.controller.trust.alpha > 0.0
                    && beta > 0.0
                    && !self.controller.certificate.lyapunov_veto
                    && self.commit_cooldown_remaining == 0
                    && !self.defensive_mode
                    && self.controller.model_confidence >= self.commit_threshold.clamp(0.0, 1.0);
                if allowed {
                    self.pending_policy = crate::types::L2Policy {
                        prior: self.controller.prior(),
                        beta,
                        alpha: self.controller.trust.alpha,
                        committed: true,
                        expert: selected,
                        expert_probabilities: probabilities,
                        gate_state: self.controller.gate_state,
                        confidence: self.controller.model_confidence,
                    };
                    modulation_committed = true;
                    modulation_gain = self.controller.trust.alpha;
                    self.commit_cooldown_remaining = self.commit_cooldown_steps;
                    self.awaiting_consequence = true;
                } else if self.controller.certificate.lyapunov_veto {
                    rejection_reason = Some("lyapunov_veto".into());
                } else if self.controller.trust.alpha <= 0.0 || beta <= 0.0 {
                    rejection_reason = Some("trust_or_confidence_zero".into());
                } else if self.commit_cooldown_remaining != 0 {
                    rejection_reason = Some("commit_cooldown".into());
                } else if self.defensive_mode {
                    rejection_reason = Some("defensive_mode".into());
                } else {
                    rejection_reason = Some("commit_threshold".into());
                }
            }
        } else {
            self.controller.last_selected_expert = KEEP_L0;
            self.controller.certificate.lyapunov_veto = false;
        }
        self.spectral_history
            .push(1.0 - state.avg_prediction_error.clamp(0.0, 1.0));
        if self.spectral_history.len() > 32 {
            self.spectral_history.remove(0);
        }
        let spectral_out = self
            .spectral_gate
            .step(&self.spectral_reflector, &self.spectral_history);
        let mut score_breakdown = score_breakdown;
        score_breakdown.insert("spectral_bias".to_string(), spectral_out.bias);
        score_breakdown.insert(
            "spectral_residual".to_string(),
            spectral_out.convergence_residual,
        );
        let mut result = self.result(
            metrics,
            biases,
            constraints,
            candidate_score,
            candidate_alignment,
            score_breakdown,
            entered,
            exited,
            self.in_fixed_point,
            self.dwell_steps,
            candidate_score,
            modulation_proposed,
            modulation_committed,
            modulation_gain,
        )?;
        result.rejection_reason = rejection_reason.clone();
        self.last_rejection_reason = rejection_reason;
        result.spectral_output = Some(spectral_out);
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    fn result(
        &self,
        metrics: crate::types::MetaMetrics,
        biases: HashMap<String, f64>,
        constraints: HashMap<String, f64>,
        candidate_score: f64,
        candidate_alignment: f64,
        score_breakdown: HashMap<String, f64>,
        entered: bool,
        exited: bool,
        in_fixed_point: bool,
        dwell_steps: u64,
        _legacy_score: f64,
        modulation_proposed: bool,
        modulation_committed: bool,
        modulation_gain: f64,
    ) -> Result<L2Result> {
        Ok(L2Result {
            metrics,
            in_fixed_point,
            biases,
            constraints,
            entered_fixed_point: entered,
            exited_fixed_point: exited,
            alignment_score: candidate_score,
            candidate_alignment_score: candidate_alignment,
            dwell_steps,
            candidate_score,
            modulation_proposed,
            modulation_committed,
            modulation_gain,
            score_breakdown,
            effect_nudge: None,
            decision: None,
            dayan_residual: self.controller.residual,
            gate_state: self.controller.gate_state,
            alpha: self.controller.trust.alpha,
            beta: if self.controller.gate_state == L2GateState::Dormant
                || self.controller.last_selected_expert == KEEP_L0
                || !self.pending_policy.committed
            {
                0.0
            } else {
                self.controller.beta()
            },
            expert_weights: self.controller.expert_weights(),
            lyapunov_potential: self.controller.residual * self.controller.residual,
            lyapunov_veto: self.controller.certificate.lyapunov_veto,
            meta_cache_hit: self.controller.meta_cache_hit,
            selected_expert: control::EXPERT_NAMES[self.controller.last_selected_expert].into(),
            l2_policy: self.pending_policy,
            applied_policy: crate::types::L2Policy::default(),
            veto_reason: self
                .controller
                .certificate
                .lyapunov_veto
                .then(|| "lyapunov_veto".into()),
            rejection_reason: None,
            spectral_output: None,
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
    use super::{control, L2MetaLayer};
    use crate::types::{L2Policy, RuleEffect, SystemState};
    use hashbrown::HashMap;

    #[test]
    fn repeated_surprise_warm_starts_recovered_expert() {
        let mut layer = L2MetaLayer::new();
        layer.enabled = true;
        layer.commit_threshold = 0.0;
        layer.controller.calm_required = 1;
        let mut state = SystemState {
            highest_attractor: "maintain coherent self-model omega dayan".into(),
            generative_base: Some("auxiliary_base".into()),
            avg_prediction_error: 0.1,
            total_steps: 12,
            ..SystemState::default()
        };
        state
            .active_code
            .insert("cognitive_rules".into(), "self bias damp".into());
        for _ in 0..5 {
            layer.controller.feedback_settled = false;
            layer.controller.observe(
                control::ControlFeedback {
                    error: 0.1,
                    reward: 1.0,
                    rollback: false,
                    consequence: 0.1,
                    kl_policy: [0.8, 0.2],
                },
                L2Policy::default(),
            );
        }
        layer.controller.feedback_settled = false;
        layer.controller.observe(
            control::ControlFeedback {
                error: 0.9,
                reward: 1.0,
                rollback: false,
                consequence: 0.0,
                kl_policy: [0.8, 0.2],
            },
            L2Policy::default(),
        );
        assert!(layer.controller.surprise_started);
        let _ = layer.step(
            &state,
            HashMap::new(),
            HashMap::new(),
            &RuleEffect::default(),
        );
        for _ in 0..5 {
            layer.controller.residual = 0.1;
            layer.controller.feedback_settled = false;
            layer.controller.observe(
                control::ControlFeedback {
                    error: 0.1,
                    reward: 1.0,
                    rollback: false,
                    consequence: 0.1,
                    kl_policy: [0.8, 0.2],
                },
                L2Policy::default(),
            );
        }
        assert!(
            layer.controller.cache_prepared,
            "gate={:?} cooldown={} surprise={} residual={} start={}",
            layer.controller.gate_state,
            layer.controller.trust.cooldown_remaining,
            layer.controller.surprise_started,
            layer.controller.residual,
            layer.controller.episode_start_residual,
        );
        assert!(
            !layer.controller.cache.entries.is_empty(),
            "gate={:?} prepared={} residual={} start={} calm={} cached={}",
            layer.controller.gate_state,
            layer.controller.cache_prepared,
            layer.controller.residual,
            layer.controller.episode_start_residual,
            layer.controller.calm_streak,
            layer.controller.cached_this_episode,
        );
        let saved_fingerprint = layer.controller.cache.entries[0].fingerprint;
        layer.controller.finish_cycle();
        layer.controller.errors.clear();
        layer.controller.rewards.clear();
        layer.controller.window_rollbacks = 0;
        layer.controller.window_total = 0;
        layer.controller.entropy_sum = 0.0;
        layer.controller.gate_state = control::L2GateState::Dormant;
        for _ in 0..5 {
            layer.controller.feedback_settled = false;
            layer.controller.observe(
                control::ControlFeedback {
                    error: 0.1,
                    reward: 1.0,
                    rollback: false,
                    consequence: 0.1,
                    kl_policy: [0.8, 0.2],
                },
                L2Policy::default(),
            );
        }
        layer.controller.feedback_settled = false;
        layer.controller.observe(
            control::ControlFeedback {
                error: 0.9,
                reward: 1.0,
                rollback: false,
                consequence: 0.0,
                kl_policy: [0.8, 0.2],
            },
            L2Policy::default(),
        );
        assert_eq!(layer.controller.fingerprint(), saved_fingerprint);
        assert!(
            layer.controller.surprise_started,
            "gate={:?} ema_error={} baseline_error={} ema_reward={} baseline_reward={}",
            layer.controller.gate_state,
            layer.controller.ema_error,
            layer.controller.baseline_error,
            layer.controller.ema_reward,
            layer.controller.baseline_reward,
        );
        let result = layer
            .step(
                &state,
                HashMap::new(),
                HashMap::new(),
                &RuleEffect::default(),
            )
            .unwrap();
        assert!(result.meta_cache_hit);
        assert!(result.alpha > 0.0);
        assert_ne!(result.selected_expert, "KEEP_L0");
    }

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
