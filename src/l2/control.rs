//! Resident L2 safety controller, implemented with `alloc` and `libm` only.
use crate::l2::meta_cache;
#[allow(unused_imports)]
use crate::prelude::*;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

pub const EXPERT_NAMES: [&str; 4] = ["KEEP_L0", "STABILIZE", "EXPLORE", "OBSERVE"];
pub const KEEP_L0: usize = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum L2GateState {
    #[default]
    Dormant,
    Active,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TrustRegion {
    pub alpha: f64,
    pub alpha_max: f64,
    pub rise_rate: f64,
    pub cooldown_steps: u32,
    pub cooldown_remaining: u32,
}
impl Default for TrustRegion {
    fn default() -> Self {
        Self {
            alpha: 0.0,
            alpha_max: 0.35,
            rise_rate: 0.08,
            cooldown_steps: 0,
            cooldown_remaining: 0,
        }
    }
}
impl TrustRegion {
    pub fn update(&mut self, active: bool, rollback: bool, reward_delta: f64, consequence: f64) {
        self.cooldown_remaining = self.cooldown_remaining.saturating_sub(1);
        if rollback || reward_delta < -0.15 {
            self.alpha = 0.0;
            self.cooldown_remaining = self.cooldown_steps.max(2);
        } else if !active {
            self.alpha = 0.0;
        } else if self.cooldown_remaining == 0 && consequence >= 0.0 {
            self.alpha = (self.alpha + self.rise_rate).min(self.alpha_max);
        } else if consequence < -0.05 {
            self.alpha = 0.0;
            self.cooldown_remaining = self.cooldown_steps;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ControlFeedback {
    pub error: f64,
    pub reward: f64,
    pub rollback: bool,
    pub consequence: f64,
    pub kl_policy: [f64; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExpertAggregator {
    pub weights: [f64; 4],
    pub cumulative_reward: [f64; 4],
    pub last_probabilities: [f64; 4],
    pub forgetting: f64,
    pub learning_rate: f64,
    pub mixing: f64,
    pub rounds: u64,
}
impl Default for ExpertAggregator {
    fn default() -> Self {
        Self {
            weights: [1.15, 1.0, 1.0, 1.0],
            cumulative_reward: [0.0; 4],
            last_probabilities: [0.25; 4],
            forgetting: 0.97,
            learning_rate: 0.35,
            mixing: 0.05,
            rounds: 0,
        }
    }
}
impl ExpertAggregator {
    pub fn probabilities(&self) -> [f64; 4] {
        let total: f64 = self.weights.iter().sum();
        if !total.is_finite() || total <= 0.0 {
            return [0.25; 4];
        }
        let mut p = [0.0; 4];
        for (i, item) in p.iter_mut().enumerate() {
            *item = 0.95 * self.weights[i] / total + self.mixing / 4.0;
        }
        p
    }
    pub fn select(&self, surprise: f64) -> usize {
        let p = self.probabilities();
        // Deterministic pseudo-random draw: the complete decision is replayable.
        let draw = libm::fabs(libm::sin(
            (self.rounds + 1) as f64 * 12.9898 + surprise * 78.233,
        ));
        let mut cumulative = 0.0;
        for (i, probability) in p.iter().enumerate() {
            cumulative += probability;
            if draw <= cumulative {
                return i;
            }
        }
        KEEP_L0
    }
    /// Inverse-propensity scoring (IPS) for the selected expert. Unselected
    /// counterfactual rewards are not observed and therefore remain zero in
    /// this sparse estimator: E[1{I=i} r/p_i | r_i] = r_i. Nonzero mixing gives
    /// every expert support, while forgetting bounds the effective horizon.
    /// Calling this standard IPS is not a claim of counterfactual observation.
    pub fn update_ips(&mut self, expert: usize, probabilities: [f64; 4], reward: f64) {
        if expert >= 4
            || !probabilities.iter().all(|p| p.is_finite() && *p > 0.0)
            || (probabilities.iter().sum::<f64>() - 1.0).abs() > 1.0e-6
        {
            return;
        }
        let reward = reward.clamp(0.0, 1.0);
        for i in 0..4 {
            self.cumulative_reward[i] *= self.forgetting;
            self.weights[i] = (self.weights[i] * self.forgetting + 1.0e-12).max(1.0e-12);
        }
        let estimate = reward / probabilities[expert].max(1.0e-6);
        self.cumulative_reward[expert] += estimate;
        self.weights[expert] *= libm::exp(self.learning_rate * estimate / 4.0);
        let maximum = self.weights.iter().copied().fold(1.0e-12, f64::max);
        for weight in &mut self.weights {
            *weight = (*weight / maximum).max(1.0e-12);
        }
        self.last_probabilities = probabilities;
        self.rounds = self.rounds.saturating_add(1);
    }
    pub fn warm_start(&mut self, mut weights: [f64; 4]) {
        // Never turn an untrusted or zero-valued cached weight into a zero
        // probability: EXP3 support is required by the IPS estimator.
        for weight in &mut weights {
            if !weight.is_finite() {
                *weight = 1.0e-12;
            }
            *weight = weight.max(1.0e-12);
        }
        self.weights = weights;
        self.last_probabilities = self.probabilities();
    }
    /// Diagnostic discounted-Hedge envelope, in reward units, under bounded
    /// rewards and a comparator with `switches` changes. It is not an
    /// unconditional dynamic-regret proof for the closed-loop environment:
    /// IPS variance, model error, and environmental feedback are additional
    /// assumptions that cannot be inferred from a benchmark score.
    pub fn tracking_regret_bound(&self, switches: u64) -> f64 {
        let horizon = self.rounds.max(1) as f64;
        let eta = self.learning_rate.max(1.0e-6);
        let effective_horizon = 1.0 / (1.0 - self.forgetting.clamp(0.0, 0.999999));
        libm::log(4.0) / eta
            + effective_horizon * eta / 8.0
            + (switches as f64 + 1.0) * (1.0 - self.forgetting) * horizon
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct StabilityCertificate {
    pub potential: f64,
    pub predicted_potential: f64,
    pub gamma: f64,
    pub epsilon: f64,
    pub lyapunov_veto: bool,
}
impl Default for StabilityCertificate {
    fn default() -> Self {
        Self {
            potential: 1.0,
            predicted_potential: 1.0,
            gamma: 0.05,
            epsilon: 0.01,
            lyapunov_veto: false,
        }
    }
}
impl StabilityCertificate {
    pub fn check(&self, residual: f64, predicted_residual: f64) -> Self {
        let potential = residual * residual;
        let predicted_potential = predicted_residual * predicted_residual;
        let veto = !potential.is_finite()
            || !predicted_potential.is_finite()
            || predicted_potential > (1.0 - self.gamma) * potential + self.epsilon;
        Self {
            potential,
            predicted_potential,
            lyapunov_veto: veto,
            ..*self
        }
    }
}

/// Exact frozen solution of E[R] - beta KL(pi || prior) for two actions.
pub fn kl_regularized_policy(rewards: [f64; 2], prior: [f64; 2], beta: f64) -> [f64; 2] {
    if beta <= 0.0 {
        return if rewards[1] > rewards[0] {
            [0.0, 1.0]
        } else {
            [1.0, 0.0]
        };
    }
    let prior = [prior[0].max(1.0e-8), prior[1].max(1.0e-8)];
    let total = prior[0] + prior[1];
    let scaled = [
        rewards[0] / beta + libm::log(prior[0] / total),
        rewards[1] / beta + libm::log(prior[1] / total),
    ];
    let max = scaled[0].max(scaled[1]);
    let exp = [libm::exp(scaled[0] - max), libm::exp(scaled[1] - max)];
    [exp[0] / (exp[0] + exp[1]), exp[1] / (exp[0] + exp[1])]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ResidentController {
    pub gate_state: L2GateState,
    /// Normal recovery is finalized at consequence settlement. Until then
    /// the commit snapshot remains valid for execution and feedback.
    pub retire_requested: bool,
    pub feedback_settled: bool,
    pub trust: TrustRegion,
    pub experts: ExpertAggregator,
    pub certificate: StabilityCertificate,
    pub cache: meta_cache::MetaCache,
    pub baseline_error: f64,
    pub ema_error: f64,
    pub ema_reward: f64,
    pub baseline_reward: f64,
    pub observations: u64,
    pub error_margin: f64,
    pub reward_drop_margin: f64,
    pub rollback_wakeup: u32,
    pub rollback_streak: u32,
    pub calm_streak: u32,
    pub calm_required: u32,
    pub max_error_history: usize,
    pub errors: Vec<f64>,
    pub rewards: Vec<f64>,
    pub window_rollbacks: u32,
    pub window_total: u32,
    pub beta_max: f64,
    pub meta_cache_hit: bool,
    /// True only on the transition into ACTIVE, including a rollback wakeup.
    pub surprise_started: bool,
    pub cache_prepared: bool,
    pub cache_prepared_fingerprint: Option<meta_cache::ContextFingerprint>,
    pub surprise_fingerprint: Option<meta_cache::ContextFingerprint>,
    pub episode_start_residual: f64,
    pub cached_this_episode: bool,
    pub entropy_sum: f64,
    pub last_selected_expert: usize,
    pub last_reward: f64,
    pub residual: f64,
    /// Residual relaxation state and its most recent frozen target. These
    /// fields retain their v4 names for snapshot compatibility.
    pub dayan_state: f64,
    pub dayan_relaxation: f64,
    pub residual_ema: f64,
    pub previous_error: f64,
    pub residual_slope: f64,
    pub residual_control_gain: f64,
    pub model_confidence: f64,
    pub veto_count: u64,
    pub cache_hits: u64,
    pub best_residual: f64,
    pub initial_residual: f64,
}
impl Default for ResidentController {
    fn default() -> Self {
        Self {
            gate_state: L2GateState::Dormant,
            retire_requested: false,
            feedback_settled: false,
            trust: TrustRegion {
                cooldown_steps: 3,
                ..TrustRegion::default()
            },
            experts: ExpertAggregator::default(),
            certificate: StabilityCertificate::default(),
            cache: meta_cache::MetaCache::default(),
            baseline_error: 0.0,
            ema_error: 0.0,
            ema_reward: 0.5,
            baseline_reward: 0.5,
            observations: 0,
            error_margin: 0.04,
            reward_drop_margin: 0.18,
            rollback_wakeup: 2,
            rollback_streak: 0,
            calm_streak: 0,
            calm_required: 3,
            max_error_history: 16,
            errors: Vec::new(),
            rewards: Vec::new(),
            window_rollbacks: 0,
            window_total: 0,
            beta_max: 2.0,
            meta_cache_hit: false,
            surprise_started: false,
            cache_prepared: false,
            cache_prepared_fingerprint: None,
            surprise_fingerprint: None,
            episode_start_residual: 1.0,
            cached_this_episode: false,
            entropy_sum: 0.0,
            last_selected_expert: KEEP_L0,
            last_reward: 0.5,
            residual: 1.0,
            dayan_state: 0.0,
            dayan_relaxation: 0.35,
            residual_ema: 1.0,
            previous_error: 0.0,
            residual_slope: 0.0,
            residual_control_gain: 0.25,
            model_confidence: 0.0,
            veto_count: 0,
            cache_hits: 0,
            best_residual: 1.0,
            initial_residual: 1.0,
        }
    }
}
impl ResidentController {
    /// Consume exactly one outcome for the policy that acted before this call.
    /// `executed_expert = KEEP_L0` means no intervention was applied and the
    /// reward update uses L0's own selection propensity (IPS still works).
    pub fn observe(&mut self, feedback: ControlFeedback, policy: crate::types::L2Policy) {
        if self.feedback_settled {
            return;
        }
        self.feedback_settled = true;
        self.meta_cache_hit = false;
        if !(feedback.error.is_finite()
            && feedback.reward.is_finite()
            && feedback.consequence.is_finite()
            && feedback
                .kl_policy
                .iter()
                .all(|p| p.is_finite() && *p >= 0.0))
        {
            self.retire_requested = true;
            self.trust.alpha = 0.0;
            self.trust.cooldown_remaining = self.trust.cooldown_steps.max(2);
            return;
        }
        let error = feedback.error.clamp(0.0, 1.0);
        if self.observations > 0 {
            let error_delta = error - self.previous_error;
            let target_delta = self.residual_ema - self.previous_error;
            self.residual_slope =
                (0.8 * self.residual_slope + 0.2 * (target_delta - error_delta)).clamp(-0.5, 0.5);
            self.model_confidence =
                (1.0 - (self.residual_slope - target_delta).abs()).clamp(0.0, 1.0);
        } else {
            self.model_confidence = 1.0;
        }
        self.previous_error = error;
        if self.observations == 0 {
            self.baseline_error = error;
            self.baseline_reward = feedback.reward;
            self.ema_error = error;
            self.ema_reward = feedback.reward;
        } else {
            self.ema_error = 0.75 * self.ema_error + 0.25 * error;
            self.ema_reward = 0.80 * self.ema_reward + 0.20 * feedback.reward;
            self.baseline_error = 0.98 * self.baseline_error + 0.02 * error;
            self.baseline_reward = 0.98 * self.baseline_reward + 0.02 * feedback.reward;
        }
        self.observations = self.observations.saturating_add(1);
        self.errors.push(error);
        self.rewards.push(feedback.reward.clamp(0.0, 1.0));
        let bounded = self.max_error_history.max(2);
        if self.errors.len() > bounded {
            self.errors.remove(0);
        }
        if self.rewards.len() > bounded {
            self.rewards.remove(0);
        }
        self.window_total = self.window_total.saturating_add(1);
        self.window_rollbacks = self
            .window_rollbacks
            .saturating_add(u32::from(feedback.rollback));
        self.rollback_streak = if feedback.rollback {
            self.rollback_streak.saturating_add(1)
        } else {
            0
        };
        let entropy = feedback
            .kl_policy
            .iter()
            .filter(|p| **p > 0.0)
            .map(|p| -p * libm::log(*p))
            .sum::<f64>()
            / libm::log(2.0);
        self.entropy_sum += entropy;
        let policy = policy.effective();
        let expert = policy.expert;
        let probabilities = policy.expert_probabilities;
        self.experts
            .update_ips(expert, probabilities, feedback.reward);

        let rollback_surprise = self.rollback_streak >= self.rollback_wakeup;
        let surprise = self.ema_error > self.baseline_error + self.error_margin
            || self.ema_reward < self.baseline_reward - self.reward_drop_margin
            || rollback_surprise;
        self.surprise_started = false;
        if surprise && self.gate_state == L2GateState::Dormant {
            self.gate_state = L2GateState::Active;
            self.surprise_started = true;
            self.calm_streak = 0;
            self.cache_prepared = false;
            self.surprise_fingerprint = Some(self.fingerprint());
            self.episode_start_residual = self.residual;
            self.cached_this_episode = false;
        }
        self.trust.update(
            self.gate_state == L2GateState::Active,
            feedback.rollback,
            feedback.reward - self.last_reward,
            feedback.consequence,
        );
        self.last_reward = feedback.reward;
        if self.gate_state == L2GateState::Active && self.observations >= 4 {
            // A changed regime may have a new normal reward/error level. Once
            // the actual fixed-point residual is low and stable, raw baseline
            // offsets must not keep the controller awake forever.
            let recovered_residual =
                self.residual <= 0.18 && self.residual < self.episode_start_residual - 0.01;
            // Recovery can mean either convergence from an actual perturbation
            // or confirmation that the original alarm has subsided. Requiring
            // a further decrease from an already tiny residual would keep a
            // healthy controller permanently ACTIVE.
            let calm_observation =
                !surprise && self.residual <= 0.18 && feedback.consequence >= -0.05;
            let recovery = !rollback_surprise && (recovered_residual || calm_observation);
            self.calm_streak = if recovery {
                self.calm_streak.saturating_add(1)
            } else {
                0
            };
            if recovery && !self.cached_this_episode && self.cache_prepared {
                let fingerprint = self
                    .surprise_fingerprint
                    .unwrap_or_else(|| self.fingerprint());
                self.cache.store(
                    fingerprint,
                    self.experts.weights,
                    self.trust.alpha,
                    1.0 + (self.initial_residual - self.residual).max(0.0),
                );
                self.cached_this_episode = true;
            }
            if self.calm_streak >= self.calm_required {
                self.retire_requested = true;
            }
        }
    }
    /// Complete a normal retirement only after the acting policy's feedback
    /// has been consumed. An unused pending commit keeps the gate snapshot
    /// valid until the next action and its consequence have both settled.
    pub fn finish_cycle(&mut self) {
        if !self.feedback_settled {
            return;
        }
        if self.retire_requested {
            self.gate_state = L2GateState::Dormant;
            self.calm_streak = 0;
            self.trust.alpha = 0.0;
            self.retire_requested = false;
            self.cached_this_episode = false;
        }
        self.feedback_settled = false;
        self.window_rollbacks = 0;
        self.window_total = 0;
        self.entropy_sum = 0.0;
        self.meta_cache_hit = false;
    }
    pub fn fingerprint(&self) -> meta_cache::ContextFingerprint {
        meta_cache::ContextFingerprint::from_window(
            &self.errors,
            self.window_rollbacks,
            self.window_total,
            self.entropy_sum / self.window_total.max(1) as f64,
        )
    }
    pub fn prior(&self) -> [f64; 2] {
        match self.last_selected_expert {
            1 => [0.90, 0.10],
            2 => [0.10, 0.90],
            3 => [0.65, 0.35],
            _ => [0.5, 0.5],
        }
    }
    pub fn beta(&self) -> f64 {
        if self.gate_state == L2GateState::Dormant
            || self.last_selected_expert == KEEP_L0
            || self.retire_requested
        {
            0.0
        } else {
            self.beta_max * self.trust.alpha / self.trust.alpha_max.max(1.0e-9)
                * self.model_confidence
        }
    }
    /// Predict one relaxation step from the current frozen target, then add
    /// the declared expert's worst-case bounded disturbance. `error` is the
    /// observation associated with the latest target; alpha only bounds
    /// uncertainty, it is not treated as a guaranteed error reduction.
    ///
    /// This is a model-based one-step check, not a claim that an expert can
    /// control the environment. For a frozen target, STABILIZE (which makes no
    /// additional perturbation in this model) recovers exactly the contraction
    /// `r+ = (1 - lambda) r`; EXPLORE/OBSERVE pay a transparent risk budget.
    pub fn predicted_residual(&self, error: f64, alpha: f64, expert: usize) -> f64 {
        let disturbance = match expert {
            1 => 0.0,
            2 => self.residual_control_gain.abs(),
            3 => 0.35 * self.residual_control_gain.abs(),
            _ => return f64::INFINITY,
        };
        if !error.is_finite() {
            return f64::INFINITY;
        }
        let lambda = self.dayan_relaxation.clamp(0.01, 1.0);
        let innovation =
            (self.residual_slope.abs() + (self.ema_error - error).abs()).clamp(0.0, 1.0);
        let disturbance_term = 0.2 * lambda * innovation + alpha.clamp(0.0, 1.0) * disturbance;
        let predicted = (1.0 - lambda) * self.residual + disturbance_term;
        if predicted.is_finite() {
            predicted
        } else {
            f64::INFINITY
        }
    }
    /// One explicit fixed-point iteration on the residual:
    /// `r+ = (1-lambda) r + lambda |target - previous_target|`.
    /// The latest observation supplies the omega=dayan target. For a frozen
    /// target and 0 < lambda <= 1 this is a contraction, hence V=r^2 decreases
    /// by (1-lambda)^2. On a regime change the target may move, which is why a
    /// separate single-step Lyapunov certificate guards every commit.
    pub fn update_dayan(&mut self, target_alignment: f64) -> f64 {
        let target = target_alignment.clamp(0.0, 1.0);
        if self.observations == 0 {
            self.residual_ema = target;
            self.initial_residual = 1.0;
            self.best_residual = 1.0;
        }
        let lambda = self.dayan_relaxation.clamp(0.01, 1.0);
        self.dayan_state =
            (1.0 - lambda) * self.dayan_state + lambda * (target - self.residual_ema).abs();
        self.dayan_state = self.dayan_state.clamp(0.0, 1.0);
        self.residual = self.dayan_state;
        self.residual_ema = target;
        self.best_residual = self.best_residual.min(self.residual);
        self.residual
    }
    pub fn expert_weights(&self) -> HashMap<String, f64> {
        let p = self.experts.probabilities();
        let mut result = HashMap::new();
        for (name, probability) in EXPERT_NAMES.iter().zip(p) {
            result.insert((*name).into(), probability);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::L2Policy;

    #[test]
    fn trust_is_zero_outside_active_and_resets_on_rollback() {
        let mut trust = TrustRegion::default();
        trust.update(false, false, 0.0, 1.0);
        assert_eq!(trust.alpha, 0.0);
        trust.update(true, false, 0.0, 1.0);
        assert!(trust.alpha > 0.0);
        trust.update(true, true, 0.0, 1.0);
        assert_eq!(trust.alpha, 0.0);
        assert!(trust.cooldown_remaining > 0);
    }

    #[test]
    fn ips_uses_logged_propensity_and_supports_every_expert() {
        let mut experts = ExpertAggregator::default();
        let probabilities = experts.probabilities();
        let before = experts.cumulative_reward[2];
        experts.update_ips(2, probabilities, 0.5);
        assert!((experts.cumulative_reward[2] - 0.5 / probabilities[2]).abs() < 1e-10);
        assert!(experts.probabilities().iter().all(|p| *p > 0.0));
        assert!(experts.tracking_regret_bound(2).is_finite());
        assert_eq!(before, 0.0);
    }

    #[test]
    fn lyapunov_vetoes_an_increasing_prediction() {
        let certificate = StabilityCertificate::default();
        assert!(certificate.check(0.5, 0.7).lyapunov_veto);
        assert!(!certificate.check(0.5, 0.4).lyapunov_veto);
    }

    #[test]
    fn kl_policy_matches_closed_form_and_zero_beta_is_greedy() {
        let policy = kl_regularized_policy([0.2, 0.8], [0.8, 0.2], 0.5);
        let total = 0.8 * libm::exp(0.2 / 0.5) + 0.2 * libm::exp(0.8 / 0.5);
        assert!((policy[0] - 0.8 * libm::exp(0.2 / 0.5) / total).abs() < 1e-12);
        assert!((policy.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        assert_eq!(
            kl_regularized_policy([0.2, 0.8], [1.0, 0.0], 0.0),
            [0.0, 1.0]
        );
    }

    #[test]
    fn malformed_feedback_fails_closed_without_learning() {
        let mut controller = ResidentController {
            gate_state: L2GateState::Active,
            trust: TrustRegion {
                alpha: 0.2,
                ..TrustRegion::default()
            },
            ..ResidentController::default()
        };
        controller.observe(
            ControlFeedback {
                error: f64::NAN,
                reward: 1.0,
                rollback: false,
                consequence: 0.0,
                kl_policy: [0.5, 0.5],
            },
            L2Policy::default(),
        );
        assert_eq!(controller.observations, 0);
        assert_eq!(controller.experts.rounds, 0);
        assert_eq!(controller.trust.alpha, 0.0);
        assert!(controller.retire_requested);
        controller.finish_cycle();
        assert_eq!(controller.gate_state, L2GateState::Dormant);
    }

    #[test]
    fn prediction_contracts_without_disturbance_and_charges_expert_risk() {
        let controller = ResidentController {
            residual: 0.4,
            dayan_state: 0.4,
            residual_ema: 0.6,
            ema_error: 0.2,
            ..ResidentController::default()
        };
        let predicted = controller.predicted_residual(0.2, 0.2, 1);
        assert!((predicted - 0.65 * 0.4).abs() < 1e-12);
        assert!(!controller.certificate.check(0.4, predicted).lyapunov_veto);
        let risky = controller.predicted_residual(0.2, 1.0, 2);
        assert!(risky > predicted);
        assert!(controller.certificate.check(0.4, risky).lyapunov_veto);
        assert!(controller
            .predicted_residual(0.2, 0.2, KEEP_L0)
            .is_infinite());
    }

    #[test]
    fn retirement_waits_for_feedback_settlement() {
        let mut controller = ResidentController {
            gate_state: L2GateState::Active,
            retire_requested: true,
            trust: TrustRegion {
                alpha: 0.2,
                ..TrustRegion::default()
            },
            ..ResidentController::default()
        };
        controller.finish_cycle();
        assert_eq!(controller.gate_state, L2GateState::Active);
        assert!(controller.retire_requested);
        controller.feedback_settled = true;
        controller.finish_cycle();
        assert_eq!(controller.gate_state, L2GateState::Dormant);
        assert!(!controller.retire_requested);
        assert_eq!(controller.trust.alpha, 0.0);
    }

    #[test]
    fn repeated_feedback_is_idempotent() {
        let mut controller = ResidentController::default();
        let feedback = ControlFeedback {
            error: 0.4,
            reward: 0.8,
            rollback: false,
            consequence: 0.1,
            kl_policy: [0.8, 0.2],
        };
        controller.observe(feedback, L2Policy::default());
        let observations = controller.observations;
        let rounds = controller.experts.rounds;
        let ema = controller.ema_error;
        controller.observe(feedback, L2Policy::default());
        assert_eq!(controller.observations, observations);
        assert_eq!(controller.experts.rounds, rounds);
        assert_eq!(controller.ema_error, ema);
    }

    #[test]
    fn fixed_point_residual_contracts_for_frozen_target() {
        let mut controller = ResidentController {
            residual: 0.8,
            dayan_state: 0.8,
            residual_ema: 0.8,
            ..ResidentController::default()
        };
        let mut previous = f64::INFINITY;
        for step in 0..8 {
            let residual = controller.update_dayan(0.8);
            assert!(
                residual < previous,
                "step={step} residual={residual} previous={previous} state={} target={} lambda={}",
                controller.dayan_state,
                controller.residual_ema,
                controller.dayan_relaxation,
            );
            controller.observe(
                ControlFeedback {
                    error: 0.1,
                    reward: 1.0,
                    rollback: false,
                    consequence: 0.1,
                    kl_policy: [1.0, 0.0],
                },
                L2Policy::default(),
            );
            controller.finish_cycle();
            previous = residual;
        }
        assert!(previous < 0.03);
    }

    #[test]
    fn gate_wakes_on_error_and_returns_to_dormant_after_recovery() {
        fn feedback(error: f64, reward: f64) -> ControlFeedback {
            ControlFeedback {
                error,
                reward,
                rollback: false,
                consequence: 0.0,
                kl_policy: [1.0, 0.0],
            }
        }
        let mut controller = ResidentController::default();
        let policy = L2Policy::default();
        for _ in 0..5 {
            controller.observe(feedback(0.1, 1.0), policy);
            controller.finish_cycle();
            controller.residual = 0.3;
            controller.episode_start_residual = 0.6;
        }
        controller.observe(feedback(0.9, 0.0), policy);
        assert_eq!(controller.gate_state, L2GateState::Active);
        assert!(controller.surprise_started);
        controller.residual = 0.6;
        controller.episode_start_residual = 0.6;
        for _ in 0..=controller.calm_required {
            controller.residual = 0.1;
            controller.observe(feedback(0.1, 1.0), policy);
            controller.finish_cycle();
        }
        // A new request must not be finalized before the acting policy's own
        // feedback has settled. In this loop the previous request was already
        // consumed; the next recovery observation starts a fresh cycle.
        assert!(controller.retire_requested || controller.gate_state == L2GateState::Dormant);
        controller.finish_cycle();
        assert_eq!(
            controller.gate_state,
            L2GateState::Dormant,
            "ema={} baseline={} reward={} reward_base={} residual={} calm={}",
            controller.ema_error,
            controller.baseline_error,
            controller.ema_reward,
            controller.baseline_reward,
            controller.residual,
            controller.calm_streak
        );
        assert_eq!(controller.trust.alpha, 0.0);
    }
}
