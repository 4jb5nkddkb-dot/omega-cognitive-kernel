//! TOML configuration for product-facing Kernel knobs.

#[allow(unused_imports)]
use crate::prelude::*;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// When the assistant/kernel may escalate beyond the user's local frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InterventionMode {
    /// DX / tool friction: allow path alternatives after friction threshold.
    #[default]
    Exploration,
    /// High-stakes domains: answer in-frame only; never decide for the user.
    Strict,
}

/// Highest intervention the system is allowed to take under current mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InterventionLevel {
    /// No unsolicited guidance.
    Silent,
    /// Stay inside the user's stated problem frame.
    #[default]
    InFrame,
    /// May propose an alternative path (e.g. Codespaces vs local MSVC).
    SuggestAlternative,
    /// Must not decide; only checklist / refuse-to-decide.
    RefuseToDecide,
}

/// Strategy-governance knobs (friction + risk domain).
///
/// Additive for 0.4.x — all fields optional with safe defaults.
/// See `docs/INTERVENTION_POLICY.md`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InterventionPolicy {
    /// exploration = DX efficiency; strict = high-stakes governance.
    pub mode: InterventionMode,
    /// Cap on how far the system may escalate under this mode.
    pub max_level: InterventionLevel,
    /// Consecutive similar env/build failures before suggesting an alternative path.
    /// Only honored when `mode = exploration` and `max_level` allows it.
    pub friction_threshold: u32,
    /// If true, emit audit-friendly notes when escalation is suppressed or allowed.
    pub audit_escalations: bool,
}

impl Default for InterventionPolicy {
    fn default() -> Self {
        Self {
            mode: InterventionMode::Exploration,
            max_level: InterventionLevel::SuggestAlternative,
            friction_threshold: 2,
            audit_escalations: true,
        }
    }
}

impl InterventionPolicy {
    /// Effective max level after applying mode constraints.
    pub fn effective_max_level(&self) -> InterventionLevel {
        match self.mode {
            InterventionMode::Strict => match self.max_level {
                InterventionLevel::Silent | InterventionLevel::InFrame => self.max_level,
                // Strict mode never allows unsolicited path-alternatives as auto-decide.
                InterventionLevel::SuggestAlternative | InterventionLevel::RefuseToDecide => {
                    InterventionLevel::RefuseToDecide
                }
            },
            InterventionMode::Exploration => self.max_level,
        }
    }

    /// Whether friction streak may trigger a path-alternative suggestion.
    pub fn may_suggest_alternative(&self, consecutive_failures: u32) -> bool {
        matches!(self.mode, InterventionMode::Exploration)
            && matches!(
                self.effective_max_level(),
                InterventionLevel::SuggestAlternative
            )
            && consecutive_failures >= self.friction_threshold
    }
}

/// Persistence privacy controls. Redaction is opt-in to preserve legacy output.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PrivacyConfig {
    /// Replace known user-provided goal/task/checkpoint-label text in persisted output.
    pub redact_user_content: bool,
}

/// Serializable product config (omega.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KernelConfig {
    /// Path for state.json (relative or absolute).
    pub state_path: Option<String>,
    pub autosave: bool,
    pub auto_checkpoint_on_patch: bool,
    /// L1 reflector: mean-error threshold that can trigger a patch.
    pub error_threshold: f64,
    /// L1 reflector: minimum steps between accepted patches.
    pub min_steps_between_patches: u64,
    /// L0 constraint: max_risk used in decision scoring.
    pub max_risk: f64,
    /// Optional initial goal string for CLI convenience (not enforced by Kernel).
    pub default_goal: Option<String>,
    /// Optional strategy-governance policy (friction + domain mode).
    pub intervention: InterventionPolicy,
    /// Master switch for autonomous cognitive code / rule evolution (L1 patches).
    /// Default **false** — self-modification only when explicitly enabled.
    pub self_modify_enabled: bool,
    #[serde(default)]
    pub l1_shadow_mode: bool,
    /// If true, every step appends a full trajectory record to trajectory.jsonl
    /// beside the state file. Default true for auditability.
    pub dump_trajectory: bool,
    /// Optional privacy controls for user-provided text persisted to disk.
    pub privacy: PrivacyConfig,
    /// Hysteresis and consequence costs for UCB-backed bandit decisions.
    pub bandit: crate::types::BanditConfig,
    /// Resident L2 controller. Monitoring is always on; bounded interventions
    /// are enabled by default and can be disabled with `l2.enabled = false`.
    pub l2: L2Config,
}

/// Pure-numeric resident-control parameters. Bounds are checked when applied
/// to the controller so untrusted configuration cannot invalidate the safety
/// certificate or EXP3 support assumptions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct L2Config {
    pub enabled: bool,
    pub commit_threshold: f64,
    pub commit_cooldown_steps: u64,
    pub negative_reward_required: u32,
    pub alpha_max: f64,
    pub alpha_rise_rate: f64,
    pub trust_cooldown_steps: u32,
    pub error_margin: f64,
    pub reward_drop_margin: f64,
    pub rollback_wakeup: u32,
    pub calm_required: u32,
    pub window_size: usize,
    pub beta_max: f64,
    pub lyapunov_gamma: f64,
    pub lyapunov_epsilon: f64,
    pub hedge_forgetting: f64,
    pub hedge_learning_rate: f64,
    pub dayan_relaxation: f64,
    pub meta_cache_capacity: usize,
    pub meta_cache_hit_threshold: f64,
}

impl Default for L2Config {
    fn default() -> Self {
        Self {
            enabled: true,
            commit_threshold: 0.82,
            commit_cooldown_steps: 5,
            negative_reward_required: 3,
            alpha_max: 0.35,
            alpha_rise_rate: 0.08,
            trust_cooldown_steps: 3,
            error_margin: 0.04,
            reward_drop_margin: 0.18,
            rollback_wakeup: 2,
            calm_required: 3,
            window_size: 16,
            beta_max: 2.0,
            lyapunov_gamma: 0.05,
            lyapunov_epsilon: 0.01,
            hedge_forgetting: 0.97,
            hedge_learning_rate: 0.35,
            dayan_relaxation: 0.35,
            meta_cache_capacity: 16,
            meta_cache_hit_threshold: 0.78,
        }
    }
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

impl L2Config {
    pub fn apply_to(&self, layer: &mut crate::l2::L2MetaLayer) {
        layer.enabled = self.enabled;
        layer.commit_threshold = finite_or(self.commit_threshold, 0.82).clamp(0.0, 1.0);
        layer.commit_cooldown_steps = self.commit_cooldown_steps.min(10_000);
        layer.commit_cooldown_remaining = layer
            .commit_cooldown_remaining
            .min(layer.commit_cooldown_steps);
        layer.negative_reward_required = self.negative_reward_required.max(1);
        let controller = &mut layer.controller;
        controller.trust.alpha_max = finite_or(self.alpha_max, 0.35).clamp(0.0, 0.35);
        controller.trust.rise_rate = finite_or(self.alpha_rise_rate, 0.08).clamp(0.0, 0.35);
        controller.trust.cooldown_steps = self.trust_cooldown_steps.min(10_000);
        controller.trust.alpha = controller.trust.alpha.min(controller.trust.alpha_max);
        controller.error_margin = finite_or(self.error_margin, 0.04).clamp(0.0, 1.0);
        controller.reward_drop_margin = finite_or(self.reward_drop_margin, 0.18).clamp(0.0, 1.0);
        controller.rollback_wakeup = self.rollback_wakeup.max(1);
        controller.calm_required = self.calm_required.max(1);
        controller.max_error_history = self.window_size.clamp(2, 1024);
        controller.errors.truncate(controller.max_error_history);
        controller.rewards.truncate(controller.max_error_history);
        controller.beta_max = finite_or(self.beta_max, 2.0).clamp(0.0, 100.0);
        controller.certificate.gamma = finite_or(self.lyapunov_gamma, 0.05).clamp(0.0, 1.0);
        controller.certificate.epsilon = finite_or(self.lyapunov_epsilon, 0.01).clamp(0.0, 1.0);
        controller.experts.forgetting = finite_or(self.hedge_forgetting, 0.97).clamp(0.0, 0.999999);
        controller.experts.learning_rate =
            finite_or(self.hedge_learning_rate, 0.35).clamp(0.0, 1.0);
        controller.dayan_relaxation = finite_or(self.dayan_relaxation, 0.35).clamp(0.01, 1.0);
        controller.cache.capacity = self.meta_cache_capacity.clamp(1, 256);
        controller.cache.entries.truncate(controller.cache.capacity);
        controller.cache.hit_threshold =
            finite_or(self.meta_cache_hit_threshold, 0.78).clamp(0.0, 1.0);
        if !self.enabled {
            controller.gate_state = crate::l2::control::L2GateState::Dormant;
            controller.retire_requested = false;
            controller.trust.alpha = 0.0;
            layer.pending_policy = crate::types::L2Policy::default();
            layer.applied_policy = crate::types::L2Policy::default();
        }
    }
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            state_path: None,
            autosave: true,
            auto_checkpoint_on_patch: true,
            error_threshold: 0.30,
            min_steps_between_patches: 3,
            max_risk: 0.65,
            default_goal: None,
            intervention: InterventionPolicy::default(),
            self_modify_enabled: false,
            l1_shadow_mode: false,
            dump_trajectory: true,
            privacy: PrivacyConfig::default(),
            bandit: crate::types::BanditConfig::default(),
            l2: L2Config::default(),
        }
    }
}

impl KernelConfig {
    pub fn resolved_state_path(&self) -> PathBuf {
        self.state_path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(crate::persist::DEFAULT_STATE_PATH))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l2_config_bounds_nonfinite_values_and_preserves_bandit_settings() {
        let mut layer = crate::l2::L2MetaLayer::new();
        let config = L2Config {
            enabled: false,
            commit_threshold: f64::NAN,
            commit_cooldown_steps: u64::MAX,
            alpha_max: f64::NAN,
            alpha_rise_rate: f64::INFINITY,
            error_margin: f64::NAN,
            reward_drop_margin: f64::NEG_INFINITY,
            beta_max: f64::NAN,
            lyapunov_gamma: f64::NAN,
            lyapunov_epsilon: f64::INFINITY,
            hedge_forgetting: f64::NAN,
            hedge_learning_rate: f64::INFINITY,
            dayan_relaxation: f64::NAN,
            meta_cache_hit_threshold: f64::NAN,
            ..L2Config::default()
        };
        config.apply_to(&mut layer);
        assert_eq!(layer.commit_threshold, 0.82);
        assert_eq!(layer.commit_cooldown_steps, 10_000);
        assert_eq!(layer.controller.trust.alpha_max, 0.35);
        assert_eq!(layer.controller.trust.rise_rate, 0.08);
        assert_eq!(layer.controller.error_margin, 0.04);
        assert_eq!(layer.controller.reward_drop_margin, 0.18);
        assert_eq!(layer.controller.beta_max, 2.0);
        assert_eq!(layer.controller.certificate.gamma, 0.05);
        assert_eq!(layer.controller.certificate.epsilon, 0.01);
        assert_eq!(layer.controller.experts.forgetting, 0.97);
        assert_eq!(layer.controller.experts.learning_rate, 0.35);
        assert_eq!(layer.controller.dayan_relaxation, 0.35);
        assert_eq!(layer.controller.cache.hit_threshold, 0.78);
        assert!(!layer.enabled);
        assert!(!layer.pending_policy.committed);
        assert!(!layer.applied_policy.committed);
    }

    #[test]
    fn intervention_strict_blocks_alternative() {
        let p = InterventionPolicy {
            mode: InterventionMode::Strict,
            max_level: InterventionLevel::SuggestAlternative,
            friction_threshold: 1,
            audit_escalations: true,
        };
        assert!(!p.may_suggest_alternative(5));
        assert_eq!(p.effective_max_level(), InterventionLevel::RefuseToDecide);
    }

    #[test]
    fn intervention_exploration_allows_after_threshold() {
        let p = InterventionPolicy::default();
        assert!(!p.may_suggest_alternative(1));
        assert!(p.may_suggest_alternative(2));
    }
}
