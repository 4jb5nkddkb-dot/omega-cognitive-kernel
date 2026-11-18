//! TOML configuration support (requires `toml` feature).

pub use crate::config::KernelConfig;
use crate::error::{OmegaError, Result};
use std::path::Path;
use toml;

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
        assert!(!c2.self_modify_enabled);
        assert!(c2.dump_trajectory);
    }

    #[test]
    fn parse_partial() {
        let s = r#"
max_risk = 0.4
error_threshold = 0.25
autosave = false
self_modify_enabled = true
"#;
        let c = KernelConfig::from_toml(s).unwrap();
        assert!((c.max_risk - 0.4).abs() < 1e-9);
        assert!((c.error_threshold - 0.25).abs() < 1e-9);
        assert!(!c.autosave);
        assert!(c.auto_checkpoint_on_patch);
        assert_eq!(
            c.intervention.mode,
            crate::config::InterventionMode::Exploration
        );
        assert!(c.self_modify_enabled);
    }

    #[test]
    fn parse_intervention_table() {
        let s = r#"
[intervention]
mode = "strict"
max_level = "refuse_to_decide"
friction_threshold = 5
"#;
        let c = KernelConfig::from_toml(s).unwrap();
        assert_eq!(c.intervention.mode, crate::config::InterventionMode::Strict);
        assert_eq!(
            c.intervention.max_level,
            crate::config::InterventionLevel::RefuseToDecide
        );
        assert_eq!(c.intervention.friction_threshold, 5);
    }

    #[test]
    fn intervention_exploration_allows_after_threshold() {
        let s = r#"
[intervention]
mode = "exploration"
max_level = "suggest_alternative"
friction_threshold = 2
"#;
        let c = KernelConfig::from_toml(s).unwrap();
        assert_eq!(
            c.intervention.mode,
            crate::config::InterventionMode::Exploration
        );
        assert_eq!(
            c.intervention.max_level,
            crate::config::InterventionLevel::SuggestAlternative
        );
        assert_eq!(c.intervention.friction_threshold, 2);
    }

    #[test]
    fn intervention_strict_blocks_alternative() {
        let s = r#"
[intervention]
mode = "strict"
max_level = "in_frame"
friction_threshold = 1
"#;
        let c = KernelConfig::from_toml(s).unwrap();
        assert_eq!(c.intervention.mode, crate::config::InterventionMode::Strict);
        assert_eq!(
            c.intervention.max_level,
            crate::config::InterventionLevel::InFrame
        );
        assert_eq!(c.intervention.friction_threshold, 1);
    }

    #[test]
    fn roundtrip_preserves_intervention() {
        let mut c = KernelConfig::default();
        c.intervention.mode = crate::config::InterventionMode::Strict;
        c.intervention.max_level = crate::config::InterventionLevel::RefuseToDecide;
        c.intervention.friction_threshold = 3;
        let s = toml::to_string_pretty(&c).unwrap();
        let c2 = KernelConfig::from_toml(&s).unwrap();
        assert_eq!(
            c2.intervention.mode,
            crate::config::InterventionMode::Strict
        );
        assert_eq!(
            c2.intervention.max_level,
            crate::config::InterventionLevel::RefuseToDecide
        );
        assert_eq!(c2.intervention.friction_threshold, 3);
    }
}
