//! Public product API (S2 + S3 hardening + adapter injection + config).

use crate::audit::{AuditEvent, AuditLog};
use crate::checkpoint::{CheckpointMeta, CheckpointStore};
use crate::config::KernelConfig;
use crate::error::{OmegaError, Result};
use crate::l0::decision::ConstrainedDecisionMaker;
use crate::l0::executor::{Executor, SandboxedExecutor};
use crate::l0::observer::{MultiModalObserver, Observer};
use crate::l0::predictor::ModelBasedPredictor;
use crate::l0::L0WorldLayer;
use crate::l1::learner::Learner;
use crate::l1::L1CognitiveLayer;
use crate::l2::L2MetaLayer;
use crate::memory::MemoryFacade;
use crate::orchestrator::{OmegaOrchestrator, StepResult};
use crate::persist::{
    resolve_state_path, KernelSnapshot, CURRENT_SCHEMA_VERSION, DEFAULT_STATE_PATH,
};
use crate::safety;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Default concrete kernel type (simulation observer + sandboxed executor).
pub type DefaultKernel = Kernel<MultiModalObserver, SandboxedExecutor>;

/// Product-facing kernel. Generic over world I/O so callers can inject adapters.
pub struct Kernel<O = MultiModalObserver, E = SandboxedExecutor>
where
    O: Observer,
    E: Executor,
{
    pub inner: OmegaOrchestrator<O, ModelBasedPredictor, ConstrainedDecisionMaker, E>,
    pub state_path: PathBuf,
    pub autosave: bool,
    pub auto_checkpoint_on_patch: bool,
    /// When true, append full step trajectory to trajectory.jsonl beside state.
    pub dump_trajectory: bool,
}

#[derive(Debug, Clone, Serialize)]
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
}

#[derive(Debug, Clone, Serialize)]
pub struct RunReport {
    pub steps: u64,
    pub patches_accepted: u64,
    pub early_avg_err: f64,
    pub final_avg_err: f64,
    pub final_damping: f64,
    pub error_decreased: bool,
    pub last_actions: Vec<String>,
}

/// Offline summary of a `trajectory.jsonl` file (no world re-execution).
#[derive(Debug, Clone, Serialize)]
pub struct TrajectoryReplaySummary {
    pub path: String,
    pub rows: usize,
    pub patches: usize,
    pub first_avg_err: Option<f64>,
    pub last_avg_err: Option<f64>,
    pub action_counts: Vec<(String, usize)>,
    /// Last N step lines (step, action, patched, avg_err) for operator view.
    pub tail: Vec<TrajectoryReplayRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrajectoryReplayRow {
    pub step: u64,
    pub action: String,
    pub patched: bool,
    pub avg_err: Option<f64>,
    pub goal: Option<String>,
}

fn default_allowed_actions() -> Vec<String> {
    vec![
        "noop".into(),
        "observe".into(),
        "explore".into(),
        "exploit".into(),
        "stabilize".into(),
    ]
}

/// Read and summarize a trajectory.jsonl without running the kernel loop.
pub fn replay_trajectory_file(path: &Path, tail_n: usize) -> Result<TrajectoryReplaySummary> {
    use std::fs;
    if !path.exists() {
        return Err(OmegaError::Persist(format!(
            "trajectory not found: {}",
            path.display()
        )));
    }
    let data = fs::read_to_string(path).map_err(|e| OmegaError::Persist(e.to_string()))?;
    let mut rows = 0usize;
    let mut patches = 0usize;
    let mut first_avg_err = None;
    let mut last_avg_err = None;
    let mut actions: HashMap<String, usize> = HashMap::new();
    let mut tail_buf: Vec<TrajectoryReplayRow> = Vec::new();

    for line in data.lines().filter(|l| !l.trim().is_empty()) {
        let v: serde_json::Value =
            serde_json::from_str(line).map_err(|e| OmegaError::Persist(e.to_string()))?;
        rows += 1;
        let step = v.get("step").and_then(|x| x.as_u64()).unwrap_or(0);
        let action = v
            .get("action")
            .and_then(|x| x.as_str())
            .unwrap_or("-")
            .to_string();
        let patched = v.get("patched").and_then(|x| x.as_bool()).unwrap_or(false);
        if patched {
            patches += 1;
        }
        let avg_err = v.get("avg_err").and_then(|x| x.as_f64());
        if first_avg_err.is_none() {
            first_avg_err = avg_err;
        }
        if avg_err.is_some() {
            last_avg_err = avg_err;
        }
        *actions.entry(action.clone()).or_insert(0) += 1;
        let goal = v
            .get("goal")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string());
        tail_buf.push(TrajectoryReplayRow {
            step,
            action,
            patched,
            avg_err,
            goal,
        });
    }

    let mut action_counts: Vec<_> = actions.into_iter().collect();
    action_counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let tail = if tail_buf.len() > tail_n {
        tail_buf.split_off(tail_buf.len() - tail_n)
    } else {
        tail_buf
    };

    Ok(TrajectoryReplaySummary {
        path: path.display().to_string(),
        rows,
        patches,
        first_avg_err,
        last_avg_err,
        action_counts,
        tail,
    })
}

impl Kernel<MultiModalObserver, SandboxedExecutor> {
    pub fn new() -> Self {
        Self::with_state_path(DEFAULT_STATE_PATH)
    }

    pub fn with_state_path(path: impl Into<PathBuf>) -> Self {
        let l0 = L0WorldLayer {
            observer: MultiModalObserver::new(),
            predictor: ModelBasedPredictor,
            decision: ConstrainedDecisionMaker,
            executor: SandboxedExecutor::new(default_allowed_actions()),
            memory: MemoryFacade::new(),
            learner: Learner::default(),
        };
        Self {
            inner: OmegaOrchestrator {
                l0,
                l1: L1CognitiveLayer::new(),
                l2: L2MetaLayer::new(),
                biases: HashMap::new(),
                constraints: safety::default_constraints(),
                total_steps: 0,
            },
            state_path: path.into(),
            autosave: true,
            auto_checkpoint_on_patch: true,
            dump_trajectory: true,
        }
    }

    pub fn open(path: Option<&str>) -> Result<Self> {
        let path = resolve_state_path(path);
        if path.exists() {
            let mut k = Self::with_state_path(path.clone());
            k.load()?;
            Ok(k)
        } else {
            Ok(Self::with_state_path(path))
        }
    }

    /// Build default kernel and apply TOML config (state path, thresholds, flags).
    pub fn from_config_path(config_path: impl AsRef<Path>) -> Result<Self> {
        let cfg = KernelConfig::load(config_path)?;
        let mut k = Self::with_state_path(cfg.resolved_state_path());
        k.apply_config(&cfg);
        if k.state_path.exists() {
            k.load()?;
        }
        Ok(k)
    }
}

impl<O, E> Kernel<O, E>
where
    O: Observer,
    E: Executor,
{
    pub fn with_adapters(observer: O, executor: E) -> Self {
        Self::with_adapters_and_path(observer, executor, DEFAULT_STATE_PATH)
    }

    pub fn with_adapters_and_path(observer: O, executor: E, path: impl Into<PathBuf>) -> Self {
        let l0 = L0WorldLayer {
            observer,
            predictor: ModelBasedPredictor,
            decision: ConstrainedDecisionMaker,
            executor,
            memory: MemoryFacade::new(),
            learner: Learner::default(),
        };
        Self {
            inner: OmegaOrchestrator {
                l0,
                l1: L1CognitiveLayer::new(),
                l2: L2MetaLayer::new(),
                biases: HashMap::new(),
                constraints: safety::default_constraints(),
                total_steps: 0,
            },
            state_path: path.into(),
            autosave: true,
            auto_checkpoint_on_patch: true,
            dump_trajectory: true,
        }
    }

    /// Apply product knobs from config without replacing adapters.
    pub fn apply_config(&mut self, cfg: &KernelConfig) {
        if let Some(ref p) = cfg.state_path {
            self.state_path = PathBuf::from(p);
        }
        self.autosave = cfg.autosave;
        self.auto_checkpoint_on_patch = cfg.auto_checkpoint_on_patch;
        self.dump_trajectory = cfg.dump_trajectory;
        self.inner.l1.reflector.error_threshold = cfg.error_threshold;
        self.inner.l1.reflector.min_steps_between_patches = cfg.min_steps_between_patches;
        self.inner.l1.self_modify_enabled = cfg.self_modify_enabled;
        self.inner
            .constraints
            .insert("max_risk".into(), cfg.max_risk);
    }

    fn audit(&self) -> AuditLog {
        AuditLog::beside_state(&self.state_path)
    }

    fn checkpoints(&self) -> CheckpointStore {
        CheckpointStore::beside_state(&self.state_path)
    }

    fn trajectory_path(&self) -> PathBuf {
        let mut p = self.state_path.clone();
        p.set_file_name("trajectory.jsonl");
        p
    }

    /// Summarize this kernel's trajectory.jsonl (offline).
    pub fn replay_trajectory(&self, tail_n: usize) -> Result<TrajectoryReplaySummary> {
        replay_trajectory_file(&self.trajectory_path(), tail_n)
    }

    fn append_trajectory(&self, goal: &str, result: &StepResult) {
        if !self.dump_trajectory {
            return;
        }
        use std::io::Write;
        let path = self.trajectory_path();
        let rec = serde_json::json!({
            "ts": chrono::Utc::now().to_rfc3339(),
            "step": self.inner.total_steps,
            "goal": goal,
            "action": result.trajectory.actions.first().map(|a| a.name.clone()),
            "prediction_errors": result.trajectory.prediction_errors,
            "patched": result.l1.patched,
            "patch_reason": result.l1.reason,
            "effect_damping": result.l1.effect_damping,
            "l2_in_fp": result.l2.in_fixed_point,
            "l2_align": result.l2.alignment_score,
            "avg_err": self.inner.l0.memory.cas.avg_prediction_error,
            "self_modify_enabled": self.inner.l1.self_modify_enabled,
        });
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = writeln!(f, "{}", rec);
        }
    }

    pub fn step(&mut self, goal: &str) -> Result<StepResult> {
        let pre_snap = if self.auto_checkpoint_on_patch {
            Some(self.snapshot())
        } else {
            None
        };
        let result = self.inner.run_once(goal)?;
        let action = result
            .trajectory
            .actions
            .first()
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "-".into());
        let avg_err = self.inner.l0.memory.cas.avg_prediction_error;
        let damping = self.inner.l0.memory.cognitive_effect.error_damping;
        let _ = self.audit().append(AuditEvent::Step {
            step: self.inner.total_steps,
            goal: goal.into(),
            action,
            avg_err,
            damping,
        });
        self.append_trajectory(goal, &result);
        if result.l1.patched {
            if let (Some(snap), Some(d)) = (pre_snap, result.l1.effect_damping) {
                if let Ok(meta) = self.checkpoints().save(
                    "pre-patch",
                    self.inner.total_steps.saturating_sub(1),
                    snap.cas_avg_prediction_error,
                    snap.cognitive_effect.error_damping,
                    &snap,
                ) {
                    let _ = self.audit().append(AuditEvent::Checkpoint {
                        checkpoint_id: meta.id,
                        step: meta.step,
                        label: meta.label,
                    });
                }
                let _ = self.audit().append(AuditEvent::PatchAccepted {
                    step: self.inner.total_steps,
                    patch_id: result.l1.patch_id.unwrap_or_else(Uuid::nil),
                    damping: d,
                    description: "cognitive_rules".into(),
                });
            }
        } else if let Some(reason) = &result.l1.reason {
            let _ = self.audit().append(AuditEvent::PatchRejected {
                step: self.inner.total_steps,
                reason: reason.clone(),
            });
        }
        if self.autosave {
            let _ = self.save();
        }
        Ok(result)
    }

    pub fn run(&mut self, goal: &str, steps: u32) -> Result<RunReport> {
        let mut patches = 0u64;
        let mut actions = Vec::new();
        let mut early = 0.0;
        for i in 0..steps {
            let r = self.step(goal)?;
            if i == 0 {
                early = self.inner.l0.memory.cas.avg_prediction_error;
            }
            if r.l1.patched {
                patches += 1;
            }
            if let Some(a) = r.trajectory.actions.first() {
                actions.push(a.name.clone());
            }
        }
        let final_err = self.inner.l0.memory.cas.avg_prediction_error;
        Ok(RunReport {
            steps: steps as u64,
            patches_accepted: patches,
            early_avg_err: early,
            final_avg_err: final_err,
            final_damping: self.inner.l0.memory.cognitive_effect.error_damping,
            error_decreased: final_err < early,
            last_actions: actions,
        })
    }

    pub fn status(&self) -> KernelStatus {
        let cases = self.inner.l0.memory.case.retrieve("", 10_000).len();
        let strategies = self.inner.l0.memory.strategy.retrieve("", 10_000).len();
        let checkpoint_count = self.checkpoints().list().map(|v| v.len()).unwrap_or(0);
        let effect = &self.inner.l0.memory.cognitive_effect;
        let max_risk = self
            .inner
            .constraints
            .get("max_risk")
            .copied()
            .unwrap_or(0.65);
        KernelStatus {
            schema_version: CURRENT_SCHEMA_VERSION,
            total_steps: self.inner.total_steps,
            avg_prediction_error: self.inner.l0.memory.cas.avg_prediction_error,
            adaptation_score: self.inner.l0.memory.cas.adaptation_score,
            error_damping: effect.error_damping,
            explore_bonus: effect.explore_bonus,
            risk_tolerance: effect.risk_tolerance,
            active_modules: self.inner.l1.store.active.len(),
            strategy_count: strategies,
            case_count: cases,
            state_path: self.state_path.display().to_string(),
            checkpoint_count,
            in_fixed_point: self.inner.l2.in_fixed_point,
            dwell_steps: self.inner.l2.dwell_steps,
            last_alignment_score: self.inner.l2.last_alignment_score,
            max_risk,
            self_modify_enabled: self.inner.l1.self_modify_enabled,
        }
    }

    pub fn snapshot(&self) -> KernelSnapshot {
        let (cases, cap) = self.inner.l0.memory.case.snapshot();
        let (active, history) = self.inner.l1.store.snapshot();
        KernelSnapshot {
            version: CURRENT_SCHEMA_VERSION,
            total_steps: self.inner.total_steps,
            biases: self.inner.biases.clone(),
            constraints: self.inner.constraints.clone(),
            cas_total_steps: self.inner.l0.memory.cas.total_steps,
            cas_avg_prediction_error: self.inner.l0.memory.cas.avg_prediction_error,
            cas_adaptation_score: self.inner.l0.memory.cas.adaptation_score,
            cognitive_effect: self.inner.l0.memory.cognitive_effect.clone(),
            cases,
            case_capacity: cap,
            strategies: self.inner.l0.memory.strategy.snapshot(),
            active_modules: active,
            patch_history: history,
        }
    }

    pub fn apply_snapshot(&mut self, snap: KernelSnapshot) {
        self.inner.total_steps = snap.total_steps;
        self.inner.biases = snap.biases;
        self.inner.constraints = snap.constraints;
        self.inner.l0.memory.cas.total_steps = snap.cas_total_steps;
        self.inner.l0.memory.cas.avg_prediction_error = snap.cas_avg_prediction_error;
        self.inner.l0.memory.cas.adaptation_score = snap.cas_adaptation_score;
        self.inner.l0.memory.cognitive_effect = snap.cognitive_effect;
        self.inner.l0.memory.case =
            crate::memory::case::CaseMemory::restore(snap.cases, snap.case_capacity);
        self.inner.l0.memory.strategy =
            crate::memory::strategy::StrategyMemory::restore(snap.strategies);
        self.inner.l1.store =
            crate::l1::store::VersionedStore::restore(snap.active_modules, snap.patch_history);
    }

    pub fn save(&self) -> Result<()> {
        self.snapshot().save_to(&self.state_path)
    }

    pub fn load(&mut self) -> Result<()> {
        let snap = KernelSnapshot::load_from(&self.state_path)?;
        self.apply_snapshot(snap);
        Ok(())
    }

    pub fn checkpoint(&self, label: &str) -> Result<CheckpointMeta> {
        let snap = self.snapshot();
        let meta = self.checkpoints().save(
            label,
            self.inner.total_steps,
            self.inner.l0.memory.cas.avg_prediction_error,
            self.inner.l0.memory.cognitive_effect.error_damping,
            &snap,
        )?;
        let _ = self.audit().append(AuditEvent::Checkpoint {
            checkpoint_id: meta.id,
            step: meta.step,
            label: meta.label.clone(),
        });
        Ok(meta)
    }

    pub fn list_checkpoints(&self) -> Result<Vec<CheckpointMeta>> {
        self.checkpoints().list()
    }

    pub fn rollback(&mut self, checkpoint_id: Uuid) -> Result<()> {
        let step_before = self.inner.total_steps;
        let snap = self.checkpoints().load_snapshot(checkpoint_id)?;
        self.apply_snapshot(snap);
        let _ = self.audit().append(AuditEvent::Rollback {
            checkpoint_id,
            step_before,
        });
        if self.autosave {
            self.save()?;
        }
        Ok(())
    }

    pub fn audit_tail(&self, n: usize) -> Result<Vec<crate::audit::AuditRecord>> {
        self.audit().tail(n)
    }

    pub fn audit_task(&self, task: &str, reward: f64, steps: u32) -> Result<()> {
        self.audit().append(AuditEvent::TaskEpisode {
            task: task.into(),
            reward,
            steps,
        })
    }
}

impl Kernel<MultiModalObserver, SandboxedExecutor> {
    pub fn reset(&mut self) -> Result<()> {
        let path = self.state_path.clone();
        let autosave = self.autosave;
        let auto_ckpt = self.auto_checkpoint_on_patch;
        let dump = self.dump_trajectory;
        *self = Self::with_state_path(path);
        self.autosave = autosave;
        self.auto_checkpoint_on_patch = auto_ckpt;
        self.dump_trajectory = dump;
        if self.state_path.exists() {
            std::fs::remove_file(&self.state_path)
                .map_err(|e| crate::error::OmegaError::Persist(e.to_string()))?;
        }
        Ok(())
    }
}

impl Default for Kernel<MultiModalObserver, SandboxedExecutor> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l0::executor::LoggingExecutor;
    use crate::l0::observer::MultiModalObserver;
    use std::env::temp_dir;
    use std::io::Write;

    #[test]
    fn run_and_persist_roundtrip() {
        let path = temp_dir().join(format!("omega-api-{}.json", Uuid::new_v4()));
        let mut k = Kernel::with_state_path(&path);
        k.autosave = true;
        assert_eq!(k.run("coherent self-model", 5).unwrap().steps, 5);
        let mut k2 = Kernel::with_state_path(&path);
        k2.load().unwrap();
        assert_eq!(k2.inner.total_steps, 5);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn checkpoint_and_rollback() {
        let path = temp_dir().join(format!("omega-rb-{}.json", Uuid::new_v4()));
        let mut k = Kernel::with_state_path(&path);
        k.autosave = false;
        k.auto_checkpoint_on_patch = false;
        k.run("g", 3).unwrap();
        let meta = k.checkpoint("mid").unwrap();
        let steps_at_ckpt = k.inner.total_steps;
        k.run("g", 4).unwrap();
        k.rollback(meta.id).unwrap();
        assert_eq!(k.inner.total_steps, steps_at_ckpt);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn with_adapters_runs() {
        let observer = MultiModalObserver::new();
        let executor = LoggingExecutor::new(default_allowed_actions()).echo_stdout(false);
        let mut k = Kernel::with_adapters(observer, executor);
        k.autosave = false;
        k.auto_checkpoint_on_patch = false;
        let report = k.run("adapter smoke", 3).unwrap();
        assert_eq!(report.steps, 3);
        assert!(!report.last_actions.is_empty());
    }

    #[test]
    fn apply_config_sets_thresholds() {
        let mut k = Kernel::new();
        let cfg = KernelConfig {
            error_threshold: 0.11,
            min_steps_between_patches: 7,
            max_risk: 0.33,
            autosave: false,
            self_modify_enabled: true,
            ..KernelConfig::default()
        };
        k.apply_config(&cfg);
        assert!((k.inner.l1.reflector.error_threshold - 0.11).abs() < 1e-9);
        assert_eq!(k.inner.l1.reflector.min_steps_between_patches, 7);
        assert!((k.inner.constraints["max_risk"] - 0.33).abs() < 1e-9);
        assert!(!k.autosave);
        assert!(k.inner.l1.self_modify_enabled);
    }

    #[test]
    fn replay_trajectory_counts_patches() {
        let path = temp_dir().join(format!("omega-traj-{}.jsonl", Uuid::new_v4()));
        let mut f = std::fs::File::create(&path).unwrap();
        let line1 = serde_json::json!({
            "step": 1,
            "action": "explore",
            "patched": false,
            "avg_err": 0.5,
            "goal": "g"
        });
        let line2 = serde_json::json!({
            "step": 2,
            "action": "stabilize",
            "patched": true,
            "avg_err": 0.3,
            "goal": "g"
        });
        writeln!(f, "{line1}").unwrap();
        writeln!(f, "{line2}").unwrap();
        let sum = replay_trajectory_file(&path, 10).unwrap();
        assert_eq!(sum.rows, 2);
        assert_eq!(sum.patches, 1);
        assert!((sum.first_avg_err.unwrap() - 0.5).abs() < 1e-9);
        assert!((sum.last_avg_err.unwrap() - 0.3).abs() < 1e-9);
        assert_eq!(sum.tail.len(), 2);
        let _ = std::fs::remove_file(&path);
    }
}
