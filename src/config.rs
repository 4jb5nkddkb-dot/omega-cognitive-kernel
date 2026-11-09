//! TOML configuration for product-facing Kernel knobs.

use crate::error::{OmegaError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
        }
    }
}

impl KernelConfig {
    pub fn from_toml(s: &str) -> Result<Self> {
        toml::from_str(s).map_err(|e| OmegaError::Config(format!("parse TOML: {e}")))
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path)
            .map_err(|e| OmegaError::Config(format!("read {}: {e}", path.display())))?;
        Self::from_toml(&raw)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let body = toml::to_string_pretty(self)
            .map_err(|e| OmegaError::Config(format!("serialize TOML: {e}")))?;
        std::fs::write(path, body)
            .map_err(|e| OmegaError::Config(format!("write {}: {e}", path.display())))?;
        Ok(())
    }

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
    fn roundtrip_defaults() {
        let c = KernelConfig::default();
        let s = toml::to_string_pretty(&c).unwrap();
        let c2 = KernelConfig::from_toml(&s).unwrap();
        assert!((c2.error_threshold - 0.30).abs() < 1e-9);
        assert_eq!(c2.min_steps_between_patches, 3);
        assert!((c2.max_risk - 0.65).abs() < 1e-9);
        assert_eq!(c2.intervention.friction_threshold, 2);
    }

    #[test]
    fn parse_partial() {
        let s = r#"
max_risk = 0.4
error_threshold = 0.25
autosave = false
"#;
        let c = KernelConfig::from_toml(s).unwrap();
        assert!((c.max_risk - 0.4).abs() < 1e-9);
        assert!((c.error_threshold - 0.25).abs() < 1e-9);
        assert!(!c.autosave);
        assert!(c.auto_checkpoint_on_patch); // default
        assert_eq!(c.intervention.mode, InterventionMode::Exploration);
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

    #[test]
    fn parse_intervention_table() {
        let s = r#"
[intervention]
mode = "strict"
max_level = "in_frame"
friction_threshold = 5
audit_escalations = false
"#;
        let c = KernelConfig::from_toml(s).unwrap();
        assert_eq!(c.intervention.mode, InterventionMode::Strict);
        assert_eq!(c.intervention.max_level, InterventionLevel::InFrame);
        assert_eq!(c.intervention.friction_threshold, 5);
        assert!(!c.intervention.audit_escalations);
    }
}
