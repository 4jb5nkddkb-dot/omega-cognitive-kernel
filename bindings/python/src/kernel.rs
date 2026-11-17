use pyo3::prelude::*;
use crate::types::{KernelStatus, RunReport, CheckpointMeta, TrajectoryReplaySummary, TrajectoryReplayRow};
use crate::error::{PythonBindingError, PyResult};
use std::path::Path;
use uuid::Uuid;

/// Python wrapper for Kernel
#[pyclass(unsendable)]
pub struct Kernel {
    inner: omega_cognitive_kernel::Kernel<
        omega_cognitive_kernel::l0::observer::MultiModalObserver,
        omega_cognitive_kernel::l0::executor::SandboxedExecutor,
    >,
}

#[pymethods]
impl Kernel {
    /// Create a new kernel with default state path
    #[staticmethod]
    pub fn new() -> PyResult<Self> {
        let kernel = omega_cognitive_kernel::Kernel::new();
        Ok(Self { inner: kernel })
    }

    /// Create a new kernel with custom state path
    #[staticmethod]
    pub fn with_state_path(state_path: &str) -> PyResult<Self> {
        let kernel = omega_cognitive_kernel::Kernel::with_state_path(state_path);
        Ok(Self { inner: kernel })
    }

    /// Open existing kernel or create new one
    #[staticmethod]
    pub fn open(state_path: Option<&str>) -> PyResult<Self> {
        let kernel = omega_cognitive_kernel::Kernel::open(state_path)
            .map_err(PythonBindingError::Kernel)?;
        Ok(Self { inner: kernel })
    }

    /// Open kernel from config file
    #[staticmethod]
    pub fn from_config_path(config_path: &str) -> PyResult<Self> {
        let kernel = omega_cognitive_kernel::Kernel::from_config_path(config_path)
            .map_err(PythonBindingError::Kernel)?;
        Ok(Self { inner: kernel })
    }

    /// Execute a single step
    pub fn step(&mut self, goal: &str) -> PyResult<RunReport> {
        let result = self.inner.step(goal).map_err(PythonBindingError::Kernel)?;
        let report = RunReport {
            steps: 1,
            patches_accepted: if result.l1.patched { 1 } else { 0 },
            early_avg_err: self.inner.inner.l0.memory.cas.avg_prediction_error,
            final_avg_err: self.inner.inner.l0.memory.cas.avg_prediction_error,
            final_damping: self.inner.inner.l0.memory.cognitive_effect.error_damping,
            error_decreased: false,
            last_actions: result.trajectory.actions.first()
                .map(|a| vec![a.name.clone()])
                .unwrap_or_default(),
        };
        Ok(report)
    }

    /// Execute multiple steps
    pub fn run(&mut self, goal: &str, steps: u32) -> PyResult<RunReport> {
        let report = self.inner.run(goal, steps).map_err(PythonBindingError::Kernel)?;
        let py_report = RunReport {
            steps: report.steps,
            patches_accepted: report.patches_accepted,
            early_avg_err: report.early_avg_err,
            final_avg_err: report.final_avg_err,
            final_damping: report.final_damping,
            error_decreased: report.error_decreased,
            last_actions: report.last_actions,
        };
        Ok(py_report)
    }

    /// Get kernel status
    pub fn status(&self) -> PyResult<KernelStatus> {
        let status = self.inner.status();
        let py_status = KernelStatus {
            schema_version: status.schema_version,
            total_steps: status.total_steps,
            avg_prediction_error: status.avg_prediction_error,
            adaptation_score: status.adaptation_score,
            error_damping: status.error_damping,
            explore_bonus: status.explore_bonus,
            risk_tolerance: status.risk_tolerance,
            active_modules: status.active_modules,
            strategy_count: status.strategy_count,
            case_count: status.case_count,
            state_path: status.state_path,
            checkpoint_count: status.checkpoint_count,
            in_fixed_point: status.in_fixed_point,
            dwell_steps: status.dwell_steps,
            last_alignment_score: status.last_alignment_score,
            max_risk: status.max_risk,
            self_modify_enabled: status.self_modify_enabled,
            l2_enabled: status.l2_enabled,
            l2_commit_cooldown: status.l2_commit_cooldown,
            last_l2_consequence_score: status.last_l2_consequence_score,
            l2_defensive_mode: status.l2_defensive_mode,
            l2_negative_consequence_streak: status.l2_negative_consequence_streak,
        };
        Ok(py_status)
    }

    /// Save kernel state to disk
    pub fn save(&mut self) -> PyResult<()> {
        self.inner.save().map_err(PythonBindingError::Kernel)?;
        Ok(())
    }

    /// Get snapshot as JSON string
    pub fn snapshot_json(&self) -> PyResult<String> {
        let json = self.inner.snapshot_json().map_err(PythonBindingError::Kernel)?;
        Ok(json)
    }

    /// Apply snapshot from JSON string
    pub fn apply_snapshot_json(&mut self, json: &str) -> PyResult<()> {
        self.inner.apply_snapshot_json(json).map_err(PythonBindingError::Kernel)?;
        Ok(())
    }

    /// Set strategy weight
    pub fn set_strategy_weight(&mut self, name: &str, weight: f64) -> PyResult<bool> {
        let ok = self.inner.set_strategy_weight(name, weight);
        Ok(ok)
    }

    /// Enable/disable L2
    pub fn set_l2_enabled(&mut self, enabled: bool) {
        self.inner.set_l2_enabled(enabled);
    }

    /// List all checkpoints
    pub fn list_checkpoints(&self) -> PyResult<Vec<CheckpointMeta>> {
        let checkpoints = self.inner.list_checkpoints().map_err(PythonBindingError::Kernel)?;
        let py_checkpoints: Vec<CheckpointMeta> = checkpoints.into_iter().map(|c| CheckpointMeta {
            id: c.id.to_string(),
            label: c.label,
            step: c.step,
            created_at: c.created_at.to_string(),
            avg_err: c.avg_err,
            damping: c.damping,
        }).collect();
        Ok(py_checkpoints)
    }

    /// Create a manual checkpoint
    pub fn create_checkpoint(&mut self, label: &str) -> PyResult<CheckpointMeta> {
        let meta = self.inner.checkpoint(label).map_err(PythonBindingError::Kernel)?;
        let py_meta = CheckpointMeta {
            id: meta.id.to_string(),
            label: meta.label,
            step: meta.step,
            created_at: meta.created_at.to_string(),
            avg_err: meta.avg_err,
            damping: meta.damping,
        };
        Ok(py_meta)
    }

    /// Rollback to checkpoint
    pub fn rollback(&mut self, checkpoint_id: &str) -> PyResult<()> {
        let uuid = Uuid::parse_str(checkpoint_id).map_err(PythonBindingError::Uuid)?;
        self.inner.rollback(uuid).map_err(PythonBindingError::Kernel)?;
        self.save()?;
        Ok(())
    }

    /// Replay trajectory
    pub fn replay_trajectory(&self, tail_n: usize) -> PyResult<TrajectoryReplaySummary> {
        let summary = self.inner.replay_trajectory(tail_n).map_err(PythonBindingError::Kernel)?;
        let py_summary = TrajectoryReplaySummary {
            path: summary.path,
            rows: summary.rows,
            patches: summary.patches,
            first_avg_err: summary.first_avg_err,
            last_avg_err: summary.last_avg_err,
            action_counts: summary.action_counts,
            tail: summary.tail.into_iter().map(|r| TrajectoryReplayRow {
                step: r.step,
                action: r.action,
                patched: r.patched,
                avg_err: r.avg_err,
                goal: r.goal,
            }).collect(),
        };
        Ok(py_summary)
    }

    /// Get state path
    pub fn state_path(&self) -> String {
        self.inner.state_path.display().to_string()
    }

    /// Get autosave setting
    pub fn autosave(&self) -> bool {
        self.inner.autosave
    }

    /// Set autosave
    pub fn set_autosave(&mut self, value: bool) {
        self.inner.autosave = value;
    }

    /// Get auto_checkpoint_on_patch setting
    pub fn auto_checkpoint_on_patch(&self) -> bool {
        self.inner.auto_checkpoint_on_patch
    }

    /// Set auto_checkpoint_on_patch
    pub fn set_auto_checkpoint_on_patch(&mut self, value: bool) {
        self.inner.auto_checkpoint_on_patch = value;
    }

    /// Get dump_trajectory setting
    pub fn dump_trajectory(&self) -> bool {
        self.inner.dump_trajectory
    }

    /// Set dump_trajectory
    pub fn set_dump_trajectory(&mut self, value: bool) {
        self.inner.dump_trajectory = value;
    }

    fn __repr__(&self) -> String {
        format!("Kernel(state_path={}, steps={})",
            self.inner.state_path.display(), self.inner.inner.total_steps)
    }
}
