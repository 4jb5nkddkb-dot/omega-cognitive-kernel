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

/// Current on-disk schema. Bump only with a migration path.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

fn default_schema_version() -> u32 {
    0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelSnapshot {
    /// On-disk schema version. Missing / 0 treated as pre-0.5 and migrated to 1.
    #[serde(default = "default_schema_version")]
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
    /// Migrate older snapshots into `CURRENT_SCHEMA_VERSION`.
    pub fn migrate(mut self) -> Result<Self> {
        if self.version > CURRENT_SCHEMA_VERSION {
            return Err(OmegaError::Persist(format!(
                "state schema version {} is newer than supported {}",
                self.version, CURRENT_SCHEMA_VERSION
            )));
        }
        if self.version < 1 {
            // v0 → v1: case.priority backfill happens in CaseMemory::restore;
            // ensure capacity is sane and stamp version.
            if self.case_capacity == 0 {
                self.case_capacity = 256;
            }
            self.version = 1;
        }
        Ok(self)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| OmegaError::Persist(e.to_string()))?;
        }
        let mut snap = self.clone();
        snap.version = CURRENT_SCHEMA_VERSION;
        let json =
            serde_json::to_string_pretty(&snap).map_err(|e| OmegaError::Persist(e.to_string()))?;
        fs::write(path, json).map_err(|e| OmegaError::Persist(e.to_string()))?;
        Ok(())
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        let data = fs::read_to_string(path).map_err(|e| OmegaError::Persist(e.to_string()))?;
        let snap: Self =
            serde_json::from_str(&data).map_err(|e| OmegaError::Persist(e.to_string()))?;
        snap.migrate()
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
        assert_eq!(loaded.version, CURRENT_SCHEMA_VERSION);
        assert_eq!(loaded.total_steps, 7);
        assert!((loaded.cognitive_effect.error_damping - 0.7).abs() < 1e-9);
        assert!((loaded.cognitive_effect.explore_bonus + 0.2).abs() < 1e-9);
        assert!((loaded.cognitive_effect.risk_tolerance - 0.4).abs() < 1e-9);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn migrates_version_zero() {
        let path = temp_dir().join(format!("omega-mig-{}.json", Uuid::new_v4()));
        // Pre-0.5 style: omit version field entirely.
        let raw = r#"{
            "total_steps": 3,
            "biases": {},
            "constraints": {"max_risk": 0.65},
            "cas_total_steps": 3,
            "cas_avg_prediction_error": 0.2,
            "cas_adaptation_score": 0.5,
            "cognitive_effect": {
                "error_damping": 1.0,
                "action_preference": {},
                "min_uncertainty": 0.05,
                "explore_bonus": 0.0,
                "risk_tolerance": 0.5
            },
            "cases": [],
            "case_capacity": 0,
            "strategies": {},
            "active_modules": {},
            "patch_history": []
        }"#;
        fs::write(&path, raw).unwrap();
        let loaded = KernelSnapshot::load_from(&path).unwrap();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.case_capacity, 256);
        assert_eq!(loaded.total_steps, 3);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn rejects_future_schema() {
        let path = temp_dir().join(format!("omega-future-{}.json", Uuid::new_v4()));
        let snap = KernelSnapshot {
            version: 99,
            total_steps: 0,
            biases: HashMap::new(),
            constraints: HashMap::new(),
            cas_total_steps: 0,
            cas_avg_prediction_error: 0.0,
            cas_adaptation_score: 0.0,
            cognitive_effect: RuleEffect::default(),
            cases: vec![],
            case_capacity: 256,
            strategies: HashMap::new(),
            active_modules: HashMap::new(),
            patch_history: vec![],
        };
        // Bypass save_to stamping by writing raw.
        fs::write(
            &path,
            serde_json::to_string(&snap).unwrap(),
        )
        .unwrap();
        let err = KernelSnapshot::load_from(&path).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("newer") || msg.contains("99"), "{msg}");
        let _ = fs::remove_file(&path);
    }
}
