//! TOML configuration for product-facing Kernel knobs.

use crate::error::{OmegaError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
        }
    }
}

impl KernelConfig {
    pub fn from_str(s: &str) -> Result<Self> {
        toml::from_str(s).map_err(|e| OmegaError::Config(format!("parse TOML: {e}")))
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path)
            .map_err(|e| OmegaError::Config(format!("read {}: {e}", path.display())))?;
        Self::from_str(&raw)
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
        let c2 = KernelConfig::from_str(&s).unwrap();
        assert!((c2.error_threshold - 0.30).abs() < 1e-9);
        assert_eq!(c2.min_steps_between_patches, 3);
        assert!((c2.max_risk - 0.65).abs() < 1e-9);
    }

    #[test]
    fn parse_partial() {
        let s = r#"
max_risk = 0.4
error_threshold = 0.25
autosave = false
"#;
        let c = KernelConfig::from_str(s).unwrap();
        assert!((c.max_risk - 0.4).abs() < 1e-9);
        assert!((c.error_threshold - 0.25).abs() < 1e-9);
        assert!(!c.autosave);
        assert!(c.auto_checkpoint_on_patch); // default
    }
}
