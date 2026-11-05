//! Disk persistence for kernel state (S2 product surface).

use crate::error::{OmegaError, Result};
use crate::memory::case::Case;
use crate::memory::strategy::Strategy;
use crate::types::{CognitivePatch, RuleEffect};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_STATE_PATH: &str = ".omega/state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelSnapshot {
    pub version: u32,
    pub total_steps: u64,
    pub biases: HashMap<String, f64>,
    pub constraints: HashMap<String, f64>,
    pub cas_total_steps: u64,
    pub cas_avg_prediction_error: f64,
    pub cas_adaptation_score: f64,
    pub cognitive_effect: RuleEffect,
    pub cases: Vec<Case>,
    pub case_capacity: usize,
    pub strategies: HashMap<String, Strategy>,
    pub active_modules: HashMap<String, String>,
    pub patch_history: Vec<(CognitivePatch, String)>,
}

impl KernelSnapshot {
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| OmegaError::Persist(e.to_string()))?;
        }
        let json =
            serde_json::to_string_pretty(self).map_err(|e| OmegaError::Persist(e.to_string()))?;
        fs::write(path, json).map_err(|e| OmegaError::Persist(e.to_string()))?;
        Ok(())
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        let data = fs::read_to_string(path).map_err(|e| OmegaError::Persist(e.to_string()))?;
        serde_json::from_str(&data).map_err(|e| OmegaError::Persist(e.to_string()))
    }
}

pub fn resolve_state_path(explicit: Option<&str>) -> PathBuf {
    explicit
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_STATE_PATH))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RuleEffect;
    use std::env::temp_dir;
    use uuid::Uuid;

    #[test]
    fn roundtrip_snapshot_file() {
        let path = temp_dir().join(format!("omega-s2-{}.json", Uuid::new_v4()));
        let snap = KernelSnapshot {
            version: 1,
            total_steps: 7,
            biases: HashMap::from([("stabilize".into(), 0.2)]),
            constraints: HashMap::from([("max_risk".into(), 0.65)]),
            cas_total_steps: 7,
            cas_avg_prediction_error: 0.33,
            cas_adaptation_score: 0.67,
            cognitive_effect: RuleEffect {
                error_damping: 0.7,
                action_preference: HashMap::from([("stabilize".into(), 1.5)]),
                min_uncertainty: 0.08,
                explore_bonus: -0.2,
                risk_tolerance: 0.4,
            },
            cases: vec![],
            case_capacity: 256,
            strategies: HashMap::new(),
            active_modules: HashMap::from([("cognitive_rules".into(), "fn x() {}".into())]),
            patch_history: vec![],
        };
        snap.save_to(&path).unwrap();
        let loaded = KernelSnapshot::load_from(&path).unwrap();
        assert_eq!(loaded.total_steps, 7);
        assert!((loaded.cognitive_effect.error_damping - 0.7).abs() < 1e-9);
        assert!((loaded.cognitive_effect.explore_bonus + 0.2).abs() < 1e-9);
        assert!((loaded.cognitive_effect.risk_tolerance - 0.4).abs() < 1e-9);
        let _ = fs::remove_file(&path);
    }
}
