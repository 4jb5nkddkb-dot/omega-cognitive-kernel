//! Disk persistence for kernel state (S2 product surface).

#[allow(unused_imports)]
use crate::prelude::*;

use crate::error::{OmegaError, Result};
use crate::memory::case::Case;
use crate::memory::strategy::Strategy;
use crate::types::{CognitivePatch, MetaMetrics, RuleEffect};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_STATE_PATH: &str = ".omega/state.json";

/// Current on-disk schema. Bump only with a migration path.
pub const CURRENT_SCHEMA_VERSION: u32 = 6;

fn default_schema_version() -> u32 {
    0
}

/// Pre-v2 default for the L1 self-modify master switch was `false` (safe).
fn default_self_modify_enabled() -> bool {
    false
}

/// Pre-v2 the cooldown counter started "warm" so the first patch could fire
/// immediately once self-modify was enabled.
fn default_steps_since_last_patch() -> u64 {
    100
}

fn default_l2_enabled() -> bool {
    // Missing legacy flags must not acquire permission to alter L0 on upgrade.
    false
}

fn default_l2_commit_threshold() -> f64 {
    0.82
}

fn default_l2_commit_cooldown() -> u64 {
    5
}

fn default_l2_negative_reward_required() -> u32 {
    3
}

fn default_l2_detector_window() -> usize {
    4
}

fn default_l2_detector_pred_threshold() -> f64 {
    0.58
}

fn default_l2_detector_self_ref_threshold() -> f64 {
    0.50
}

fn default_l2_detector_cooldown() -> usize {
    5
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalLoopState {
    #[serde(default)]
    pub last_external_step_id: Option<u64>,
    pub cooldown: crate::orchestrator::CooldownTracker,
    #[serde(default)]
    pub last_checkpoint_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelSnapshot {
    /// On-disk schema version. Missing / 0 treated as pre-0.5 and migrated up to
    /// [`CURRENT_SCHEMA_VERSION`] (currently 6) on load.
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
    #[serde(default = "default_l2_enabled")]
    pub l2_enabled: bool,
    #[serde(default)]
    pub l2_in_fixed_point: bool,
    #[serde(default)]
    pub l2_dwell_steps: u64,
    #[serde(default)]
    pub l2_last_alignment_score: f64,
    #[serde(default = "default_l2_commit_threshold")]
    pub l2_commit_threshold: f64,
    #[serde(default = "default_l2_commit_cooldown")]
    pub l2_commit_cooldown_steps: u64,
    #[serde(default)]
    pub l2_commit_cooldown_remaining: u64,
    #[serde(default)]
    pub l2_last_consequence_score: f64,
    #[serde(default)]
    pub l2_defensive_mode: bool,
    #[serde(default)]
    pub l2_negative_consequence_streak: u32,
    #[serde(default)]
    pub l2_stable_consequence_steps: u32,
    #[serde(default)]
    pub l2_reward_change_score: f64,
    #[serde(default)]
    pub l2_error_change_score: f64,
    #[serde(default)]
    pub l2_awaiting_consequence: bool,
    #[serde(default)]
    pub l2_negative_reward_streak: u32,
    #[serde(default = "default_l2_negative_reward_required")]
    pub l2_negative_reward_required: u32,
    #[serde(default)]
    pub l2_detector_history: Vec<MetaMetrics>,
    #[serde(default = "default_l2_detector_window")]
    pub l2_detector_window: usize,
    #[serde(default = "default_l2_detector_pred_threshold")]
    pub l2_detector_pred_threshold: f64,
    #[serde(default = "default_l2_detector_self_ref_threshold")]
    pub l2_detector_self_ref_threshold: f64,
    #[serde(default = "default_l2_detector_cooldown")]
    pub l2_detector_cooldown_steps: usize,
    #[serde(default)]
    pub l2_detector_cooldown_remaining: usize,
    /// L1 self-modify master switch (v2+). Restored so that enabling L1 then
    /// restarting does not silently fall back to the safe default-off.
    #[serde(default = "default_self_modify_enabled")]
    pub self_modify_enabled: bool,
    /// L1 patch cooldown counter (v2+). Restored so a restart cannot reset the
    /// min-steps-between-patches cooldown.
    #[serde(default = "default_steps_since_last_patch")]
    pub steps_since_last_patch: u64,
    /// L2 rollback policy snapshot (v2+). Restored so that after an L2 commit a
    /// restart retains the pre-commit policy needed for negative-feedback rollback.
    #[serde(default)]
    pub l2_rollback_effect: Option<RuleEffect>,
    /// Last UCB-backed L2 decision, available to status and trajectory replay.
    #[serde(default)]
    pub last_l2_decision: Option<crate::types::L2DecisionRecord>,
    /// Complete local bandit state when the kernel uses BanditUcbDecisionMaker.
    #[serde(default)]
    pub bandit_ucb: Option<crate::l0::bandit_decision::BanditUcbSnapshot>,
    #[serde(default)]
    pub bandit_config: crate::types::BanditConfig,
    #[serde(default)]
    pub l1_previous_effect: Option<RuleEffect>,
    #[serde(default)]
    pub l1_pending_rollback: bool,
    /// Resident L2 controller and one-step pending policy (v4+). Missing legacy
    /// state is dormant with zero trust, so migration cannot inject a bias.
    #[serde(default)]
    pub l2_controller: crate::l2::control::ResidentController,
    #[serde(default)]
    pub l2_pending_policy: crate::types::L2Policy,
    #[serde(default)]
    pub l2_applied_policy: crate::types::L2Policy,
    #[serde(default)]
    pub l2_pending_l1_rollback: bool,
    #[serde(default)]
    pub l2_last_reward: f64,
    /// Per-instance prioritized-replay cursor (v6+), independent of other kernels.
    #[serde(default)]
    pub case_sample_counter: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_loop: Option<ExternalLoopState>,
}

impl Default for KernelSnapshot {
    fn default() -> Self {
        Self {
            version: CURRENT_SCHEMA_VERSION,
            total_steps: 0,
            biases: HashMap::new(),
            constraints: HashMap::new(),
            cas_total_steps: 0,
            cas_avg_prediction_error: 0.0,
            cas_adaptation_score: 0.0,
            cognitive_effect: RuleEffect::default(),
            cases: Vec::new(),
            case_capacity: 256,
            strategies: HashMap::new(),
            active_modules: HashMap::new(),
            patch_history: Vec::new(),
            l2_enabled: true,
            l2_in_fixed_point: false,
            l2_dwell_steps: 0,
            l2_last_alignment_score: 0.0,
            l2_commit_threshold: default_l2_commit_threshold(),
            l2_commit_cooldown_steps: default_l2_commit_cooldown(),
            l2_commit_cooldown_remaining: 0,
            l2_last_consequence_score: 0.0,
            l2_defensive_mode: false,
            l2_negative_consequence_streak: 0,
            l2_stable_consequence_steps: 0,
            l2_reward_change_score: 0.0,
            l2_error_change_score: 0.0,
            l2_awaiting_consequence: false,
            l2_negative_reward_streak: 0,
            l2_negative_reward_required: default_l2_negative_reward_required(),
            l2_detector_history: Vec::new(),
            l2_detector_window: 4,
            l2_detector_pred_threshold: 0.58,
            l2_detector_self_ref_threshold: 0.50,
            l2_detector_cooldown_steps: 5,
            l2_detector_cooldown_remaining: 0,
            self_modify_enabled: false,
            steps_since_last_patch: default_steps_since_last_patch(),
            l2_rollback_effect: None,
            last_l2_decision: None,
            bandit_ucb: None,
            bandit_config: crate::types::BanditConfig::default(),
            l1_previous_effect: None,
            l1_pending_rollback: false,
            l2_controller: crate::l2::control::ResidentController::default(),
            l2_pending_policy: crate::types::L2Policy::default(),
            l2_applied_policy: crate::types::L2Policy::default(),
            l2_pending_l1_rollback: false,
            l2_last_reward: 0.5,
            case_sample_counter: 0,
            external_loop: None,
        }
    }
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
        if self.version < 2 {
            // v1 → v2: the new safety fields (`self_modify_enabled`,
            // `steps_since_last_patch`, `l2_rollback_effect`) are absent from v1
            // snapshots and therefore already carry their serde defaults. The
            // defaults are intentionally the *safe* pre-v2 behavior: L1 disabled
            // and a warm cooldown. Nothing to backfill beyond stamping the version.
            self.version = 2;
        }
        if self.version < 3 {
            // v3 adds optional UCB statistics and decision explanations. Their
            // defaults preserve the old policy and never enable self-modification.
            self.version = 3;
        }
        if self.version < 4 {
            // v4 adds the resident controller, bounded meta-cache and pending
            // policy. Serde defaults fail closed to DORMANT and KEEP_L0.
            self.l2_pending_policy = crate::types::L2Policy::default();
            self.l2_applied_policy = crate::types::L2Policy::default();
            self.version = 4;
        }
        if self.version < 5 {
            // v5 makes the online Dayan state, duplicate-feedback guard and
            // normalized propensity snapshots explicit. Missing v4 fields are
            // safe serde defaults; permission remains opt-in, never inferred
            // from the presence of a resident controller or a cache entry.
            self.l2_pending_policy = self.l2_pending_policy.effective();
            self.l2_applied_policy = self.l2_applied_policy.effective();
            if !self.l2_enabled {
                self.l2_pending_policy = crate::types::L2Policy::default();
                self.l2_applied_policy = crate::types::L2Policy::default();
                self.l2_controller.gate_state = crate::l2::control::L2GateState::Dormant;
                self.l2_controller.trust.alpha = 0.0;
            }
            self.version = 5;
        }
        if self.version < 6 {
            // v6 makes replay sampling local to the case-memory instance. Legacy
            // snapshots have no meaningful global cursor, so start at zero.
            self.case_sample_counter = 0;
            self.version = 6;
        }
        Ok(self)
    }

    /// Redact known user-provided goal text before writing a snapshot.
    /// Numeric learning state and schema remain unchanged.
    pub fn redact_user_content(&mut self) {
        for case in &mut self.cases {
            case.goal = "[REDACTED]".into();
        }
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            crate::private_fs::create_dir_all(parent)
                .map_err(|e| OmegaError::Persist(e.to_string()))?;
        }
        let mut snap = self.clone();
        snap.version = CURRENT_SCHEMA_VERSION;
        let json =
            serde_json::to_string_pretty(&snap).map_err(|e| OmegaError::Persist(e.to_string()))?;
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                OmegaError::Persist(format!("invalid state path: {}", path.display()))
            })?;
        let temp_path = parent.join(format!(".{file_name}.{}.tmp", crate::new_uuid()));
        let result = (|| -> Result<()> {
            use std::io::Write;
            let mut file = crate::private_fs::open_options()
                .write(true)
                .create_new(true)
                .open(&temp_path)
                .map_err(|e| OmegaError::Persist(format!("state temp create: {e}")))?;
            file.write_all(json.as_bytes())
                .and_then(|()| file.sync_all())
                .map_err(|e| OmegaError::Persist(format!("state temp write: {e}")))?;
            fs::rename(&temp_path, path)
                .map_err(|e| OmegaError::Persist(format!("state atomic replace: {e}")))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }
        result
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        let data = fs::read_to_string(path).map_err(|e| OmegaError::Persist(e.to_string()))?;
        let snap: Self =
            serde_json::from_str(&data).map_err(|e| OmegaError::Persist(e.to_string()))?;
        snap.migrate()
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|e| OmegaError::Persist(e.to_string()))
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let snap: Self =
            serde_json::from_str(json).map_err(|e| OmegaError::Persist(e.to_string()))?;
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

    #[test]
    fn new_snapshots_enable_l2_but_not_l1_self_modification() {
        let snapshot = KernelSnapshot::default();
        assert!(snapshot.l2_enabled);
        assert!(!snapshot.self_modify_enabled);
    }

    #[test]
    fn roundtrip_snapshot_file() {
        let path = temp_dir().join(format!("omega-s2-{}.json", crate::new_uuid()));
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
            ..KernelSnapshot::default()
        };
        snap.save_to(&path).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let loaded = KernelSnapshot::load_from(&path).unwrap();
        assert_eq!(loaded.version, CURRENT_SCHEMA_VERSION);
        assert_eq!(loaded.total_steps, 7);
        assert!((loaded.cognitive_effect.error_damping - 0.7).abs() < 1e-9);
        assert!((loaded.cognitive_effect.explore_bonus + 0.2).abs() < 1e-9);
        assert!((loaded.cognitive_effect.risk_tolerance - 0.4).abs() < 1e-9);
        let _ = fs::remove_file(&path);
    }

    #[test]
    #[cfg(unix)]
    fn failed_atomic_replace_preserves_previous_state() {
        let dir = temp_dir().join(format!("omega-atomic-{}", crate::new_uuid()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("marker"), b"previous-state").unwrap();

        let error = KernelSnapshot::default().save_to(&path).unwrap_err();
        assert!(error.to_string().contains("atomic replace"), "{error}");
        assert_eq!(
            fs::read_to_string(path.join("marker")).unwrap(),
            "previous-state"
        );
        assert!(fs::read_dir(&dir)
            .unwrap()
            .filter_map(std::result::Result::ok)
            .all(|entry| entry.path().extension().and_then(|x| x.to_str()) != Some("tmp")));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn migrates_version_one_to_current() {
        // A v1 snapshot lacks the v2 safety fields entirely; they must default.
        let path = temp_dir().join(format!("omega-v1-{}.json", crate::new_uuid()));
        let raw = r#"{
            "version": 1,
            "total_steps": 12,
            "biases": {},
            "constraints": {},
            "cas_total_steps": 12,
            "cas_avg_prediction_error": 0.1,
            "cas_adaptation_score": 0.9,
            "cognitive_effect": {
                "error_damping": 1.0,
                "action_preference": {},
                "min_uncertainty": 0.05,
                "explore_bonus": 0.0,
                "risk_tolerance": 0.5
            },
            "cases": [],
            "case_capacity": 256,
            "strategies": {},
            "active_modules": {},
            "patch_history": []
        }"#;
        fs::write(&path, raw).unwrap();
        let loaded = KernelSnapshot::load_from(&path).unwrap();
        assert_eq!(loaded.version, CURRENT_SCHEMA_VERSION);
        assert_eq!(loaded.total_steps, 12);
        assert!(!loaded.self_modify_enabled);
        assert_eq!(loaded.steps_since_last_patch, 100);
        assert!(loaded.l2_rollback_effect.is_none());
        assert!(loaded.last_l2_decision.is_none());
        assert_eq!(loaded.bandit_config.min_dwell_steps, 3);
        assert_eq!(
            loaded.l2_controller.gate_state,
            crate::l2::control::L2GateState::Dormant
        );
        assert_eq!(loaded.l2_controller.trust.alpha, 0.0);
        assert!(!loaded.l2_pending_policy.committed);
        assert!(!loaded.l2_applied_policy.committed);
        assert!(loaded.l2_controller.cache.entries.is_empty());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn roundtrips_v2_safety_fields() {
        let path = temp_dir().join(format!("omega-v2rt-{}.json", crate::new_uuid()));
        let mut snap = KernelSnapshot {
            total_steps: 42,
            ..KernelSnapshot::default()
        };
        snap.self_modify_enabled = true;
        snap.steps_since_last_patch = 3;
        snap.l2_rollback_effect = Some(RuleEffect {
            error_damping: 0.42,
            explore_bonus: 0.11,
            ..RuleEffect::default()
        });
        snap.save_to(&path).unwrap();
        let loaded = KernelSnapshot::load_from(&path).unwrap();
        assert_eq!(loaded.version, CURRENT_SCHEMA_VERSION);
        assert!(loaded.self_modify_enabled);
        assert_eq!(loaded.steps_since_last_patch, 3);
        let rb = loaded
            .l2_rollback_effect
            .expect("rollback effect persisted");
        assert!((rb.error_damping - 0.42).abs() < 1e-9);
        assert!((rb.explore_bonus - 0.11).abs() < 1e-9);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn migrates_version_zero() {
        let path = temp_dir().join(format!("omega-mig-{}.json", crate::new_uuid()));
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
        assert_eq!(loaded.version, CURRENT_SCHEMA_VERSION);
        assert_eq!(loaded.case_capacity, 256);
        assert_eq!(loaded.total_steps, 3);
        // v2 safety fields fall back to their safe defaults when migrating v0.
        assert!(!loaded.self_modify_enabled);
        assert!(loaded.l2_rollback_effect.is_none());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn migrates_v3_and_v4_without_inventing_l2_permission() {
        for version in [3, 4] {
            let mut value = serde_json::to_value(KernelSnapshot::default()).unwrap();
            value["version"] = serde_json::json!(version);
            value["l2_enabled"] = serde_json::json!(false);
            value["l2_controller"]["gate_state"] = serde_json::json!("ACTIVE");
            value["l2_controller"]["trust"]["alpha"] = serde_json::json!(0.9);
            value["l2_pending_policy"]["committed"] = serde_json::json!(true);
            value["l2_applied_policy"]["committed"] = serde_json::json!(true);

            let migrated = KernelSnapshot::from_json(&value.to_string()).unwrap();
            assert_eq!(migrated.version, CURRENT_SCHEMA_VERSION);
            assert!(!migrated.l2_enabled);
            assert_eq!(
                migrated.l2_controller.gate_state,
                crate::l2::control::L2GateState::Dormant
            );
            assert_eq!(migrated.l2_controller.trust.alpha, 0.0);
            assert!(!migrated.l2_pending_policy.committed);
            assert!(!migrated.l2_applied_policy.committed);
        }
    }

    #[test]
    fn legacy_decision_cannot_claim_an_l2_intervention() {
        let mut value = serde_json::to_value(crate::types::L2DecisionRecord {
            selected_arm: 0,
            selected_bias: "stabilize".into(),
            candidate_arm: 1,
            mean_reward: 0.5,
            exploration_bonus: 0.1,
            ucb_score: Some(0.5),
            candidate_ucb_score: Some(0.6),
            current_ucb_score: Some(0.5),
            reason: crate::types::L2DecisionReason::DwellHold,
            dwell_steps: 2,
            dwell_suppressed: true,
            margin_suppressed: false,
            l2_policy_applied: true,
            switched: false,
            safety_override: false,
            adjusted_reward: None,
            rollback_count: 0,
            switch_cost_applied: 0.0,
        })
        .unwrap();
        value.as_object_mut().unwrap().remove("l2_policy_applied");
        let record: crate::types::L2DecisionRecord = serde_json::from_value(value).unwrap();
        assert!(!record.l2_policy_applied);
    }

    #[test]
    fn legacy_policy_does_not_enable_spectral_weighting() {
        let mut value = serde_json::to_value(crate::types::L2Policy::default()).unwrap();
        value.as_object_mut().unwrap().remove("spectral_weighting");
        let policy: crate::types::L2Policy = serde_json::from_value(value).unwrap();
        assert!(!policy.spectral_weighting);
        let snapshot = KernelSnapshot::default();
        assert!(!snapshot.l2_pending_policy.spectral_weighting);
        assert!(!snapshot.l2_applied_policy.spectral_weighting);
        assert!(!crate::l2::L2MetaLayer::new().spectral_control_enabled);
    }

    #[test]
    fn rejects_future_schema() {
        let path = temp_dir().join(format!("omega-future-{}.json", crate::new_uuid()));
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
            ..KernelSnapshot::default()
        };
        // Bypass save_to stamping by writing raw.
        fs::write(&path, serde_json::to_string(&snap).unwrap()).unwrap();
        let err = KernelSnapshot::load_from(&path).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("newer") || msg.contains("99"), "{msg}");
        let _ = fs::remove_file(&path);
    }
}
