use crate::error::{PyResult, PythonBindingError};
use crate::types::{
    CheckpointMeta, KernelStatus, RunReport, TrajectoryReplayRow, TrajectoryReplaySummary,
};
use omega_cognitive_kernel::OmegaError;
use pyo3::prelude::*;
use uuid::Uuid;
use omega_cognitive_kernel::api::{
    ClosedLoopKernel, ClosedLoopSession as CoreClosedLoopSession, ExternalOutcome, StepTelemetry,
};
use omega_cognitive_kernel::config::{KernelConfig, LoopPreset};
use pyo3::exceptions::PyValueError;

/// Python wrapper for Kernel
#[pyclass(unsendable)]
pub struct Kernel {
    inner: omega_cognitive_kernel::Kernel<
        omega_cognitive_kernel::l0::observer::MultiModalObserver,
        omega_cognitive_kernel::l0::decision::ConstrainedDecisionMaker,
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
    #[pyo3(signature = (state_path=None))]
    pub fn open(state_path: Option<&str>) -> PyResult<Self> {
        let kernel =
            omega_cognitive_kernel::Kernel::open(state_path).map_err(PythonBindingError::Kernel)?;
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
            last_actions: result
                .trajectory
                .actions
                .first()
                .map(|a| vec![a.name.clone()])
                .unwrap_or_default(),
        };
        Ok(report)
    }

    /// Execute multiple steps
    pub fn run(&mut self, goal: &str, steps: u32) -> PyResult<RunReport> {
        let report = self
            .inner
            .run(goal, steps)
            .map_err(PythonBindingError::Kernel)?;
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
            last_l2_decision: status
                .last_l2_decision
                .and_then(|decision| serde_json::to_string(&decision).ok()),
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
        let json = self
            .inner
            .snapshot_json()
            .map_err(PythonBindingError::Kernel)?;
        Ok(json)
    }

    /// Apply snapshot from JSON string
    pub fn apply_snapshot_json(&mut self, json: &str) -> PyResult<()> {
        self.inner
            .apply_snapshot_json(json)
            .map_err(|error| match error {
                OmegaError::Persist(message) => PythonBindingError::SnapshotJson(message),
                other => PythonBindingError::Kernel(other),
            })?;
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

    /// Set the maximum audit JSONL file size in bytes for future events.
    pub fn set_audit_max_bytes(&mut self, max_bytes: u64) {
        self.inner.set_audit_max_bytes(max_bytes);
    }

    /// List all checkpoints
    pub fn list_checkpoints(&self) -> PyResult<Vec<CheckpointMeta>> {
        let checkpoints = self
            .inner
            .list_checkpoints()
            .map_err(PythonBindingError::Kernel)?;
        let py_checkpoints: Vec<CheckpointMeta> = checkpoints
            .into_iter()
            .map(|c| CheckpointMeta {
                id: c.id.to_string(),
                label: c.label,
                step: c.step,
                created_at: c.created_at.to_string(),
                avg_err: c.avg_err,
                damping: c.damping,
            })
            .collect();
        Ok(py_checkpoints)
    }

    /// Create a manual checkpoint
    pub fn create_checkpoint(&mut self, label: &str) -> PyResult<CheckpointMeta> {
        let meta = self
            .inner
            .checkpoint(label)
            .map_err(PythonBindingError::Kernel)?;
        self.save()?;
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
        self.inner
            .rollback(uuid)
            .map_err(PythonBindingError::Kernel)?;
        self.save()?;
        Ok(())
    }

    /// Replay trajectory
    pub fn replay_trajectory(&self, tail_n: usize) -> PyResult<TrajectoryReplaySummary> {
        let summary = self
            .inner
            .replay_trajectory(tail_n)
            .map_err(PythonBindingError::Kernel)?;
        let py_summary = TrajectoryReplaySummary {
            path: summary.path,
            rows: summary.rows,
            patches: summary.patches,
            first_avg_err: summary.first_avg_err,
            last_avg_err: summary.last_avg_err,
            action_counts: summary.action_counts,
            tail: summary
                .tail
                .into_iter()
                .map(|r| TrajectoryReplayRow {
                    step: r.step,
                    action: r.action,
                    patched: r.patched,
                    avg_err: r.avg_err,
                    goal: r.goal,
                })
                .collect(),
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

    /// Get the maximum trajectory JSONL file size in bytes.
    pub fn trajectory_max_bytes(&self) -> u64 {
        self.inner.trajectory_max_bytes()
    }

    /// Set the maximum trajectory JSONL file size in bytes for future records.
    pub fn set_trajectory_max_bytes(&mut self, value: u64) {
        self.inner.set_trajectory_max_bytes(value);
    }

    fn __repr__(&self) -> String {
        format!(
            "Kernel(state_path={}, steps={})",
            self.inner.state_path.display(),
            self.inner.inner.total_steps
        )
    }
}

fn parse_loop_preset(name: &str) -> pyo3::PyResult<LoopPreset> {
    match name {
        "deterministic" => Ok(LoopPreset::Deterministic),
        "hot_patcher" | "hot-patcher" => Ok(LoopPreset::HotPatcher),
        "meta_guard" | "meta-guard" => Ok(LoopPreset::MetaGuard),
        "fast_adapt" | "fast-adapt" => Ok(LoopPreset::FastAdapt),
        "steady_prior" | "steady-prior" => Ok(LoopPreset::SteadyPrior),
        "control_plane" | "control-plane" => Ok(LoopPreset::ControlPlane),
        "full_kernel" | "full-kernel" => Ok(LoopPreset::FullKernel),
        other => Err(PyValueError::new_err(format!("unknown loop preset: {other}"))),
    }
}

fn parse_external_outcome(name: &str) -> pyo3::PyResult<ExternalOutcome> {
    match name {
        "improved" | "accepted_and_improved" => Ok(ExternalOutcome::AcceptedAndImproved),
        "degraded" => Ok(ExternalOutcome::Degraded),
        "timeout" | "timeout_or_skipped" => Ok(ExternalOutcome::TimeoutOrSkipped),
        other => Err(PyValueError::new_err(format!(
            "unknown external outcome: {other}"
        ))),
    }
}

/// Python wrapper for v0.7.0 ClosedLoopSession supporting all 7 (L0, L1, L2) presets.
#[pyclass(unsendable)]
pub struct ClosedLoopSession {
    inner: CoreClosedLoopSession,
}

#[pymethods]
impl ClosedLoopSession {
    #[staticmethod]
    #[pyo3(signature = (preset, state_path=None, shadow_mode=false))]
    pub fn from_preset(
        preset: &str,
        state_path: Option<&str>,
        shadow_mode: bool,
    ) -> pyo3::PyResult<Self> {
        let parsed = parse_loop_preset(preset)?;
        let mut cfg = KernelConfig::default().with_loop_preset(parsed);
        cfg.l1_shadow_mode = shadow_mode;
        let mut kernel = match state_path {
            Some(path) => omega_cognitive_kernel::Kernel::with_state_path(path),
            None => omega_cognitive_kernel::Kernel::new(),
        };
        kernel.apply_config(&cfg);
        Ok(Self {
            inner: CoreClosedLoopSession::from_kernel(kernel, &cfg),
        })
    }

    pub fn switches_json(&self) -> pyo3::PyResult<String> {
        serde_json::to_string(&self.inner.switches())
            .map_err(|e| PyValueError::new_err(format!("serialize switches failed: {e}")))
    }

    pub fn step_internal_json(&mut self) -> pyo3::PyResult<String> {
        let directive = self.inner.step_internal().map_err(PyValueError::new_err)?;
        serde_json::to_string(&directive)
            .map_err(|e| PyValueError::new_err(format!("serialize directive failed: {e}")))
    }

    #[pyo3(signature = (step_id, prediction_error, observed_risk=0.0, signal_window=None, last_outcome=None))]
    pub fn step_external_json(
        &mut self,
        step_id: u64,
        prediction_error: f64,
        observed_risk: f64,
        signal_window: Option<Vec<f64>>,
        last_outcome: Option<&str>,
    ) -> pyo3::PyResult<String> {
        let parsed_outcome = match last_outcome {
            Some(raw) => Some(parse_external_outcome(raw)?),
            None => None,
        };
        let telemetry = StepTelemetry {
            step_id,
            prediction_error,
            observed_risk,
            signal_window: signal_window.unwrap_or_else(|| vec![prediction_error]),
            last_outcome: parsed_outcome,
        };
        let directive = self.inner.step_external(telemetry).map_err(PyValueError::new_err)?;
        serde_json::to_string(&directive)
            .map_err(|e| PyValueError::new_err(format!("serialize directive failed: {e}")))
    }
}
