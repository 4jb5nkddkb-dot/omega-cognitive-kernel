use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

/// Kernel status information
#[pyclass(get_all)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelStatus {
    pub schema_version: u32,
    pub total_steps: u64,
    pub avg_prediction_error: f64,
    pub adaptation_score: f64,
    pub error_damping: f64,
    pub explore_bonus: f64,
    pub risk_tolerance: f64,
    pub active_modules: usize,
    pub strategy_count: usize,
    pub case_count: usize,
    pub state_path: String,
    pub checkpoint_count: usize,
    pub in_fixed_point: bool,
    pub dwell_steps: u64,
    pub last_alignment_score: f64,
    pub max_risk: f64,
    pub self_modify_enabled: bool,
    pub l2_enabled: bool,
    pub l2_commit_cooldown: u64,
    pub last_l2_consequence_score: f64,
    pub l2_defensive_mode: bool,
    pub l2_negative_consequence_streak: u32,
    /// JSON-encoded latest UCB/L2 decision; None when no UCB decision exists.
    pub last_l2_decision: Option<String>,
}

#[pymethods]
impl KernelStatus {
    fn __repr__(&self) -> String {
        format!(
            "KernelStatus(total_steps={}, avg_err={:.3}, adaptation={:.3}, checkpoints={}, in_fixed_point={})",
            self.total_steps, self.avg_prediction_error, self.adaptation_score,
            self.checkpoint_count, self.in_fixed_point
        )
    }
}

/// Run report after executing multiple steps
#[pyclass(get_all)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunReport {
    pub steps: u64,
    pub patches_accepted: u64,
    pub early_avg_err: f64,
    pub final_avg_err: f64,
    pub final_damping: f64,
    pub error_decreased: bool,
    pub last_actions: Vec<String>,
}

#[pymethods]
impl RunReport {
    fn __repr__(&self) -> String {
        format!(
            "RunReport(steps={}, patches={}, err {:.3}→{:.3}, decreased={})",
            self.steps,
            self.patches_accepted,
            self.early_avg_err,
            self.final_avg_err,
            self.error_decreased
        )
    }
}

/// Strategy information
#[pyclass(get_all)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyInfo {
    pub name: String,
    pub weight: f64,
    pub success_count: u32,
    pub fail_count: u32,
    pub idle_steps: u32,
}

#[pymethods]
impl StrategyInfo {
    fn __repr__(&self) -> String {
        format!(
            "StrategyInfo(name={}, weight={:.3}, success={}, fail={})",
            self.name, self.weight, self.success_count, self.fail_count
        )
    }
}

/// Checkpoint metadata
#[pyclass(get_all)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMeta {
    pub id: String,
    pub label: String,
    pub step: u64,
    pub created_at: String,
    pub avg_err: f64,
    pub damping: f64,
}

#[pymethods]
impl CheckpointMeta {
    fn __repr__(&self) -> String {
        format!(
            "CheckpointMeta(id={}, label={}, step={}, err={:.3})",
            &self.id[..8],
            self.label,
            self.step,
            self.avg_err
        )
    }
}

/// Audit event kinds
#[pyclass(get_all)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuditEvent {
    Step {
        step: u64,
        goal: String,
        action: String,
        avg_err: f64,
        damping: f64,
    },
    PatchAccepted {
        step: u64,
        patch_id: String,
        damping: f64,
        description: String,
    },
    PatchRejected {
        step: u64,
        reason: String,
    },
    Checkpoint {
        checkpoint_id: String,
        step: u64,
        label: String,
    },
    Rollback {
        checkpoint_id: String,
        step_before: u64,
    },
    TaskEpisode {
        task: String,
        reward: f64,
        steps: u32,
    },
    ToolAllowed {
        tool: String,
    },
    ToolDenied {
        tool: String,
        reason: String,
    },
    FixedPointEntered {
        step: u64,
        alignment_score: f64,
    },
    FixedPointExited {
        step: u64,
        alignment_score: f64,
        dwell_steps: u64,
    },
    L2ModulationProposed {
        step: u64,
        score: f64,
    },
    L2ModulationCommitted {
        step: u64,
        score: f64,
        gain: f64,
    },
}

/// Audit record with timestamp
#[pyclass(get_all)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub ts: String,
    pub event: AuditEvent,
}

#[pymethods]
impl AuditRecord {
    fn __repr__(&self) -> String {
        format!("AuditRecord(ts={}, event={:?})", self.ts, self.event)
    }
}

impl From<omega_cognitive_kernel::AuditRecord> for AuditRecord {
    fn from(record: omega_cognitive_kernel::AuditRecord) -> Self {
        serde_json::from_value(serde_json::to_value(record).expect("audit record serialization"))
            .expect("compatible Python audit record")
    }
}

/// Audit summary
#[pyclass(get_all)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditSummary {
    pub total_events: usize,
    pub steps: usize,
    pub patches_accepted: usize,
    pub patches_rejected: usize,
    pub checkpoints: usize,
    pub rollbacks: usize,
    pub task_episodes: usize,
    pub tools_allowed: usize,
    pub tools_denied: usize,
    pub fixed_point_enters: usize,
    pub fixed_point_exits: usize,
    pub l2_modulation_proposals: usize,
    pub l2_modulation_commits: usize,
    pub first_ts: Option<String>,
    pub last_ts: Option<String>,
    pub last_action: Option<String>,
    pub last_avg_err: Option<f64>,
    pub last_damping: Option<f64>,
    pub action_counts: Vec<(String, usize)>,
}

#[pymethods]
impl AuditSummary {
    fn __repr__(&self) -> String {
        format!(
            "AuditSummary(events={}, steps={}, patches={}, checkpoints={})",
            self.total_events, self.steps, self.patches_accepted, self.checkpoints
        )
    }
}

/// Trajectory replay row
#[pyclass(get_all)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryReplayRow {
    pub step: u64,
    pub action: String,
    pub patched: bool,
    pub avg_err: Option<f64>,
    pub goal: Option<String>,
}

/// Trajectory replay summary
#[pyclass(get_all)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryReplaySummary {
    pub path: String,
    pub rows: usize,
    pub patches: usize,
    pub first_avg_err: Option<f64>,
    pub last_avg_err: Option<f64>,
    pub action_counts: Vec<(String, usize)>,
    pub tail: Vec<TrajectoryReplayRow>,
}
