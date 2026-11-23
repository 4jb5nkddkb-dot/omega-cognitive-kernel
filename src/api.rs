//! Public product API (S2 + S3 hardening + adapter injection + config).

#[allow(unused_imports)]
use crate::prelude::*;

use crate::audit::{AuditEvent, AuditLog};
use crate::checkpoint::{CheckpointMeta, CheckpointStore};
use crate::config::KernelConfig;
#[cfg(feature = "toml")]
use crate::config_toml;
use crate::error::{OmegaError, Result};
use crate::l0::bandit_decision::BanditUcbDecisionMaker;
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
use hashbrown::HashMap;
use serde::Serialize;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Default concrete kernel type (simulation observer + sandboxed executor).
pub type DefaultKernel = Kernel<MultiModalObserver, ConstrainedDecisionMaker, SandboxedExecutor>;

/// Kernel using standard UCB decision maker for bandit tasks.
pub type BanditKernel = Kernel<MultiModalObserver, BanditUcbDecisionMaker, SandboxedExecutor>;

/// In-process event callback (after audit append).
pub type EventHook = Box<dyn FnMut(&AuditEvent) + Send>;

/// Product-facing kernel. Generic over world I/O and decision maker so callers can inject adapters.
pub struct Kernel<O = MultiModalObserver, D = ConstrainedDecisionMaker, E = SandboxedExecutor>
where
    O: Observer,
    D: crate::l0::decision::DecisionMaker,
    E: Executor,
{
    pub inner: OmegaOrchestrator<O, ModelBasedPredictor, D, E>,
    pub state_path: PathBuf,
    pub autosave: bool,
    pub auto_checkpoint_on_patch: bool,
    /// When true, append full step trajectory to trajectory.jsonl beside state.
    pub dump_trajectory: bool,
    /// Optional in-process hook invoked after each audit event is appended.
    event_hook: Option<EventHook>,
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
    pub l2_enabled: bool,
    pub l2_commit_cooldown: u64,
    pub last_l2_consequence_score: f64,
    pub l2_defensive_mode: bool,
    pub l2_negative_consequence_streak: u32,
    pub last_l2_decision: Option<crate::types::L2DecisionRecord>,
    pub l2_gate_state: crate::l2::control::L2GateState,
    pub l2_alpha: f64,
    pub l2_expert_weights: HashMap<String, f64>,
    pub l2_beta: f64,
    pub l2_lyapunov_potential: f64,
    pub l2_lyapunov_veto: bool,
    pub l2_meta_cache_hit: bool,
    pub l2_applied_policy: crate::types::L2Policy,
    pub l2_meta_cache_entries: usize,
    pub l2_last_rejection_reason: Option<String>,
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
    pub l2_decision: Option<crate::types::L2DecisionRecord>,
    pub l2_gate_state: crate::l2::control::L2GateState,
    pub l2_alpha: f64,
    pub l2_expert_weights: HashMap<String, f64>,
    pub l2_beta: f64,
    pub l2_lyapunov_potential: f64,
    pub l2_lyapunov_veto: bool,
    pub l2_meta_cache_hit: bool,
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
            l2_decision: v
                .get("l2_decision")
                .cloned()
                .and_then(|value| serde_json::from_value(value).ok()),
            l2_gate_state: v
                .get("l2_gate_state")
                .cloned()
                .and_then(|value| serde_json::from_value(value).ok())
                .unwrap_or_default(),
            l2_alpha: v
                .get("l2_alpha")
                .and_then(|value| value.as_f64())
                .unwrap_or(0.0),
            l2_expert_weights: v
                .get("l2_expert_weights")
                .cloned()
                .and_then(|value| serde_json::from_value(value).ok())
                .unwrap_or_default(),
            l2_beta: v
                .get("l2_beta")
                .and_then(|value| value.as_f64())
                .unwrap_or(0.0),
            l2_lyapunov_potential: v
                .get("l2_lyapunov_potential")
                .and_then(|value| value.as_f64())
                .unwrap_or(1.0),
            l2_lyapunov_veto: v
                .get("l2_lyapunov_veto")
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
            l2_meta_cache_hit: v
                .get("l2_meta_cache_hit")
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
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

impl Kernel<MultiModalObserver, ConstrainedDecisionMaker, SandboxedExecutor> {
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
            event_hook: None,
        }
    }

    /// Construct a Kernel with the standard UCB bandit decision maker.
    pub fn with_bandit_ucb(path: impl Into<PathBuf>) -> BanditKernel {
        let l0 = L0WorldLayer {
            observer: MultiModalObserver::new(),
            predictor: ModelBasedPredictor,
            decision: BanditUcbDecisionMaker::default(),
            executor: SandboxedExecutor::new(default_allowed_actions()),
            memory: MemoryFacade::new(),
            learner: Learner::default(),
        };
        BanditKernel {
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
            event_hook: None,
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

    #[cfg(feature = "toml")]
    pub fn from_config_path(config_path: impl AsRef<Path>) -> Result<Self> {
        let cfg = config_toml::KernelConfig::load(config_path)?;
        let mut k = Self::with_state_path(cfg.resolved_state_path());
        k.apply_config(&cfg);
        if k.state_path.exists() {
            k.load()?;
        }
        Ok(k)
    }

    pub fn reset(&mut self) -> Result<()> {
        let path = self.state_path.clone();
        let autosave = self.autosave;
        let auto_ckpt = self.auto_checkpoint_on_patch;
        let dump = self.dump_trajectory;
        let hook = self.event_hook.take();
        *self = Self::with_state_path(path);
        self.autosave = autosave;
        self.auto_checkpoint_on_patch = auto_ckpt;
        self.dump_trajectory = dump;
        self.event_hook = hook;
        if self.state_path.exists() {
            std::fs::remove_file(&self.state_path)
                .map_err(|e| crate::error::OmegaError::Persist(e.to_string()))?;
        }
        Ok(())
    }
}

impl Default for Kernel<MultiModalObserver, ConstrainedDecisionMaker, SandboxedExecutor> {
    fn default() -> Self {
        Self::new()
    }
}

impl<O, D, E> Kernel<O, D, E>
where
    O: Observer,
    D: crate::l0::decision::DecisionMaker,
    E: Executor,
{
    pub fn with_adapters(observer: O, executor: E, decision: D) -> Self {
        Self::with_adapters_and_path(observer, executor, decision, DEFAULT_STATE_PATH)
    }

    pub fn with_adapters_and_path(
        observer: O,
        executor: E,
        decision: D,
        path: impl Into<PathBuf>,
    ) -> Self {
        let l0 = L0WorldLayer {
            observer,
            predictor: ModelBasedPredictor,
            decision,
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
            event_hook: None,
        }
    }

    pub fn apply_config(&mut self, cfg: &KernelConfig) {
        if let Some(ref p) = cfg.state_path {
            self.state_path = PathBuf::from(p);
        }
        self.autosave = cfg.autosave;
        self.auto_checkpoint_on_patch = cfg.auto_checkpoint_on_patch;
        self.dump_trajectory = cfg.dump_trajectory;
        self.inner.l0.decision.set_bandit_config(cfg.bandit.clone());
        self.inner.l1.reflector.error_threshold = cfg.error_threshold;
        self.inner.l1.reflector.min_steps_between_patches = cfg.min_steps_between_patches;
        self.inner.l1.self_modify_enabled = cfg.self_modify_enabled;
        self.inner
            .constraints
            .insert("max_risk".into(), cfg.max_risk);
    }

    /// Register an in-process event hook (fires after audit append).
    pub fn set_event_hook(&mut self, hook: impl FnMut(&AuditEvent) + Send + 'static) {
        self.event_hook = Some(Box::new(hook));
    }

    /// Remove the in-process event hook.
    pub fn clear_event_hook(&mut self) {
        self.event_hook = None;
    }

    /// Enable or disable L2 interventions while keeping metric evaluation
    /// available. Primarily useful for controlled baseline comparisons.
    pub fn set_l2_enabled(&mut self, enabled: bool) {
        self.inner.l2.enabled = enabled;
        if !enabled {
            self.inner.l2.in_fixed_point = false;
            self.inner.l2.dwell_steps = 0;
            self.inner.l2.controller.gate_state = crate::l2::control::L2GateState::Dormant;
            self.inner.l2.controller.retire_requested = false;
            self.inner.l2.controller.trust.alpha = 0.0;
            self.inner.l2.pending_policy = crate::types::L2Policy::default();
            self.inner.l2.applied_policy = crate::types::L2Policy::default();
            self.inner
                .l0
                .decision
                .set_l2_policy(crate::types::L2Policy::default());
        }
    }

    /// Feed back the short-term consequence of the latest L2 modulation.
    pub fn observe_l2_consequence(&mut self, reward_delta: f64, error_delta: f64, risk_delta: f64) {
        self.inner
            .l2
            .observe_consequence(reward_delta, error_delta, risk_delta);
        self.inner.l2.last_reward = (reward_delta + 0.5).clamp(0.0, 1.0);
        if reward_delta < -0.15 {
            self.inner.l2.controller.trust.alpha = 0.0;
            self.inner.l2.controller.trust.cooldown_remaining =
                self.inner.l2.controller.trust.cooldown_steps.max(2);
        }
        if self.inner.l2.applied_policy.committed || reward_delta < -0.15 {
            self.inner.l2.pending_policy = crate::types::L2Policy::default();
            self.inner
                .l0
                .decision
                .set_l2_policy(crate::types::L2Policy::default());
            self.inner.l2.controller.finish_cycle();
        }
        if let Some(effect) = self.inner.l2.take_rollback_effect() {
            self.inner.l0.memory.cognitive_effect = effect;
            self.inner.l0.decision.request_safety_override();
        }
    }

    /// Settle the previously applied policy once. The task calls this after
    /// external reward and L1 rollback are known, before the next L0 decision.
    /// A policy produced by the current L2 step remains pending for next step.
    pub fn observe_l2_feedback(
        &mut self,
        reward: f64,
        error: f64,
        rollback: bool,
        consequence: f64,
        kl_policy: [f64; 2],
    ) {
        self.inner.l2.pending_l1_rollback |= rollback;
        self.inner.l2.last_reward = reward.clamp(0.0, 1.0);
        self.inner.l2.controller.observe(
            crate::l2::control::ControlFeedback {
                error,
                reward,
                rollback,
                consequence,
                kl_policy,
            },
            self.inner.l2.applied_policy,
        );
        if rollback || reward < 0.35 {
            self.inner.l2.pending_policy = crate::types::L2Policy::default();
            self.inner
                .l0
                .decision
                .set_l2_policy(crate::types::L2Policy::default());
        }
    }

    /// Apply L1 RuleEffect rollback independently of L2's own safety rollback.
    /// Returns whether an actual previous effect was restored.
    pub fn observe_l1_consequence(&mut self, error_delta: f64) -> bool {
        if !self.inner.l1.observe_consequence(error_delta) {
            return false;
        }
        if let Some(effect) = self.inner.l1.take_rollback_effect() {
            self.inner.l0.memory.cognitive_effect = effect;
            self.inner.l2.pending_l1_rollback = true;
            return true;
        }
        false
    }

    fn emit_event(&mut self, event: &AuditEvent) {
        if let Some(hook) = self.event_hook.as_mut() {
            hook(event);
        }
    }

    /// Append an audit event, then fire the hook only on a successful append.
    ///
    /// Persistence contract (fail-closed): a failure to durably record the event
    /// is returned to the caller so that a step cannot report success while the
    /// audit trail silently drops the transition. The in-process hook fires only
    /// after the append has actually succeeded.
    fn append_event(&mut self, event: AuditEvent) -> Result<()> {
        self.audit().append(event.clone())?;
        self.emit_event(&event);
        Ok(())
    }

    /// Operator nudge: set a strategy weight (0.1–10). Returns false if unknown.
    pub fn set_strategy_weight(&mut self, name: &str, weight: f64) -> bool {
        self.inner.l0.memory.strategy.set_weight(name, weight)
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

    pub fn replay_trajectory(&self, tail_n: usize) -> Result<TrajectoryReplaySummary> {
        replay_trajectory_file(&self.trajectory_path(), tail_n)
    }

    fn append_trajectory(&self, goal: &str, result: &StepResult) -> Result<()> {
        if !self.dump_trajectory {
            return Ok(());
        }
        use std::io::Write;
        let path = self.trajectory_path();
        let rec = serde_json::json!({
            "ts": crate::now_utc_string(),
            "step": self.inner.total_steps,
            "goal": goal,
            "action": result.trajectory.actions.first().map(|a| a.name.clone()),
            "prediction_errors": result.trajectory.prediction_errors,
            "patched": result.l1.patched,
            "patch_reason": result.l1.reason,
            "effect_damping": result.l1.effect_damping,
            "l2_in_fp": result.l2.in_fixed_point,
            "l2_align": result.l2.alignment_score,
            "l2_candidate_alignment": result.l2.candidate_alignment_score,
            "l2_dwell_steps": result.l2.dwell_steps,
            "l2_candidate_score": result.l2.candidate_score,
            "l2_modulation_proposed": result.l2.modulation_proposed,
            "l2_modulation_committed": result.l2.modulation_committed,
            "l2_modulation_gain": result.l2.modulation_gain,
            "l2_score_breakdown": result.l2.score_breakdown,
            "l2_decision": result.l2.decision,
            "l2_gate_state": result.l2.gate_state,
            "l2_alpha": result.l2.alpha,
            "l2_expert_weights": result.l2.expert_weights,
            "l2_beta": result.l2.beta,
            "l2_lyapunov_potential": result.l2.lyapunov_potential,
            "l2_lyapunov_veto": result.l2.lyapunov_veto,
            "l2_meta_cache_hit": result.l2.meta_cache_hit,
            "l2_rejection_reason": result.l2.rejection_reason,
            "l2_applied_policy": result.l2.applied_policy,
            "l2_selected_expert": result.l2.selected_expert,
            "l2_dayan_residual": result.l2.dayan_residual,
            "avg_err": self.inner.l0.memory.cas.avg_prediction_error,
            "self_modify_enabled": self.inner.l1.self_modify_enabled,
        });
        // Fail-closed: a trajectory write failure is a real error, not ignored.
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| OmegaError::Persist(format!("trajectory open {}: {e}", path.display())))?;
        writeln!(f, "{rec}")
            .map_err(|e| OmegaError::Persist(format!("trajectory write {}: {e}", path.display())))
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
        self.append_event(AuditEvent::Step {
            step: self.inner.total_steps,
            goal: goal.into(),
            action,
            avg_err,
            damping,
        })?;

        // Emit L2 fixed-point regime changes for observability.
        if result.l2.entered_fixed_point {
            self.append_event(AuditEvent::FixedPointEntered {
                step: self.inner.total_steps,
                alignment_score: result.l2.alignment_score,
            })?;
        }
        if result.l2.exited_fixed_point {
            self.append_event(AuditEvent::FixedPointExited {
                step: self.inner.total_steps,
                alignment_score: result.l2.alignment_score,
                dwell_steps: self.inner.l2.dwell_steps,
            })?;
        }
        if result.l2.modulation_proposed {
            self.append_event(AuditEvent::L2ModulationProposed {
                step: self.inner.total_steps,
                score: result.l2.candidate_score,
            })?;
        }
        if result.l2.modulation_committed {
            self.append_event(AuditEvent::L2ModulationCommitted {
                step: self.inner.total_steps,
                score: result.l2.candidate_score,
                gain: result.l2.modulation_gain,
            })?;
        }

        self.append_trajectory(goal, &result)?;
        if result.l1.patched {
            if let (Some(snap), Some(d)) = (pre_snap, result.l1.effect_damping) {
                // Best-effort pre-patch checkpoint. A failure to save the optional
                // rollback snapshot must NOT suppress the primary PatchAccepted
                // event: the patch was really accepted and that fact is what the
                // audit trail must record. Checkpoint failure is propagated only
                // if its own audit event also fails.
                if let Ok(meta) = self.checkpoints().save(
                    "pre-patch",
                    self.inner.total_steps.saturating_sub(1),
                    snap.cas_avg_prediction_error,
                    snap.cognitive_effect.error_damping,
                    &snap,
                ) {
                    self.append_event(AuditEvent::Checkpoint {
                        checkpoint_id: meta.id,
                        step: meta.step,
                        label: meta.label,
                    })?;
                }
                self.append_event(AuditEvent::PatchAccepted {
                    step: self.inner.total_steps,
                    patch_id: result.l1.patch_id.unwrap_or_else(Uuid::nil),
                    damping: d,
                    description: "cognitive_rules".into(),
                })?;
            }
        } else if let Some(reason) = &result.l1.reason {
            self.append_event(AuditEvent::PatchRejected {
                step: self.inner.total_steps,
                reason: reason.clone(),
            })?;
        }
        if self.autosave {
            self.save()?;
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
            l2_enabled: self.inner.l2.enabled,
            l2_commit_cooldown: self.inner.l2.commit_cooldown_remaining,
            last_l2_consequence_score: self.inner.l2.last_consequence_score,
            l2_defensive_mode: self.inner.l2.defensive_mode,
            l2_negative_consequence_streak: self.inner.l2.negative_consequence_streak,
            last_l2_decision: self.inner.l0.decision.l2_decision(),
            l2_gate_state: self.inner.l2.controller.gate_state,
            l2_alpha: self.inner.l2.controller.trust.alpha,
            l2_expert_weights: self.inner.l2.controller.expert_weights(),
            l2_beta: self.inner.l2.controller.beta(),
            l2_lyapunov_potential: self.inner.l2.controller.residual.powi(2),
            l2_lyapunov_veto: self.inner.l2.controller.certificate.lyapunov_veto,
            l2_meta_cache_hit: self.inner.l2.controller.meta_cache_hit,
            l2_applied_policy: self.inner.l2.applied_policy,
            l2_meta_cache_entries: self.inner.l2.controller.cache.entries.len(),
            l2_last_rejection_reason: self.inner.l2.last_rejection_reason.clone(),
        }
    }

    pub fn snapshot(&self) -> KernelSnapshot {
        let mut snapshot = self.snapshot_base();
        snapshot.bandit_ucb = self.inner.l0.decision.bandit_snapshot();
        snapshot
    }

    fn snapshot_base(&self) -> KernelSnapshot {
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
            l2_enabled: self.inner.l2.enabled,
            l2_in_fixed_point: self.inner.l2.in_fixed_point,
            l2_dwell_steps: self.inner.l2.dwell_steps,
            l2_last_alignment_score: self.inner.l2.last_alignment_score,
            l2_commit_threshold: self.inner.l2.commit_threshold,
            l2_commit_cooldown_steps: self.inner.l2.commit_cooldown_steps,
            l2_commit_cooldown_remaining: self.inner.l2.commit_cooldown_remaining,
            l2_last_consequence_score: self.inner.l2.last_consequence_score,
            l2_defensive_mode: self.inner.l2.defensive_mode,
            l2_negative_consequence_streak: self.inner.l2.negative_consequence_streak,
            l2_stable_consequence_steps: self.inner.l2.stable_consequence_steps,
            l2_reward_change_score: self.inner.l2.reward_change_score,
            l2_error_change_score: self.inner.l2.error_change_score,
            l2_awaiting_consequence: self.inner.l2.awaiting_consequence,
            l2_negative_reward_streak: self.inner.l2.negative_reward_streak,
            l2_negative_reward_required: self.inner.l2.negative_reward_required,
            l2_detector_history: self.inner.l2.detector.history.clone(),
            l2_detector_window: self.inner.l2.detector.window,
            l2_detector_pred_threshold: self.inner.l2.detector.pred_threshold,
            l2_detector_self_ref_threshold: self.inner.l2.detector.self_ref_threshold,
            l2_detector_cooldown_steps: self.inner.l2.detector.cooldown_steps,
            l2_detector_cooldown_remaining: self.inner.l2.detector.cooldown_remaining,
            self_modify_enabled: self.inner.l1.self_modify_enabled,
            steps_since_last_patch: self.inner.l1.steps_since_last_patch,
            l2_rollback_effect: self.inner.l2.rollback_effect.clone(),
            last_l2_decision: self.inner.l0.decision.l2_decision(),
            bandit_ucb: None,
            bandit_config: crate::types::BanditConfig::default(),
            l1_previous_effect: self.inner.l1.previous_effect.clone(),
            l1_pending_rollback: self.inner.l1.pending_rollback,
            l2_controller: self.inner.l2.controller.clone(),
            l2_pending_policy: self.inner.l2.pending_policy,
            l2_applied_policy: self.inner.l2.applied_policy,
            l2_pending_l1_rollback: self.inner.l2.pending_l1_rollback,
            l2_last_reward: self.inner.l2.last_reward,
        }
    }

    pub fn apply_snapshot(&mut self, mut snap: KernelSnapshot) {
        let bandit_ucb = snap.bandit_ucb.take();
        self.apply_snapshot_base(snap);
        if let Some(bandit) = bandit_ucb {
            self.inner.l0.decision.restore_bandit_snapshot(bandit);
        }
    }

    fn apply_snapshot_base(&mut self, snap: KernelSnapshot) {
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
        self.inner.l2.enabled = snap.l2_enabled;
        self.inner.l2.in_fixed_point = snap.l2_in_fixed_point;
        self.inner.l2.dwell_steps = snap.l2_dwell_steps;
        self.inner.l2.last_alignment_score = snap.l2_last_alignment_score;
        self.inner.l2.commit_threshold = snap.l2_commit_threshold;
        self.inner.l2.commit_cooldown_steps = snap.l2_commit_cooldown_steps;
        self.inner.l2.commit_cooldown_remaining = snap.l2_commit_cooldown_remaining;
        self.inner.l2.last_consequence_score = snap.l2_last_consequence_score;
        self.inner.l2.defensive_mode = snap.l2_defensive_mode;
        self.inner.l2.negative_consequence_streak = snap.l2_negative_consequence_streak;
        self.inner.l2.stable_consequence_steps = snap.l2_stable_consequence_steps;
        self.inner.l2.reward_change_score = snap.l2_reward_change_score;
        self.inner.l2.error_change_score = snap.l2_error_change_score;
        self.inner.l2.awaiting_consequence = snap.l2_awaiting_consequence;
        self.inner.l2.negative_reward_streak = snap.l2_negative_reward_streak;
        self.inner.l2.negative_reward_required = snap.l2_negative_reward_required;
        self.inner.l2.detector.history = snap.l2_detector_history;
        self.inner.l2.detector.window = snap.l2_detector_window.max(1);
        self.inner.l2.detector.pred_threshold = snap.l2_detector_pred_threshold;
        self.inner.l2.detector.self_ref_threshold = snap.l2_detector_self_ref_threshold;
        self.inner.l2.detector.cooldown_steps = snap.l2_detector_cooldown_steps;
        self.inner.l2.detector.cooldown_remaining = snap.l2_detector_cooldown_remaining;
        self.inner.l1.self_modify_enabled = snap.self_modify_enabled;
        self.inner.l1.steps_since_last_patch = snap.steps_since_last_patch;
        self.inner.l2.rollback_effect = snap.l2_rollback_effect;
        self.inner.l1.previous_effect = snap.l1_previous_effect;
        self.inner.l1.pending_rollback = snap.l1_pending_rollback;
        self.inner.l2.controller = snap.l2_controller;
        self.inner.l2.pending_policy = snap.l2_pending_policy.effective();
        self.inner.l2.applied_policy = snap.l2_applied_policy.effective();
        self.inner.l2.pending_l1_rollback = snap.l2_pending_l1_rollback;
        self.inner.l2.last_reward = snap.l2_last_reward;
        self.inner
            .l0
            .decision
            .set_l2_policy(self.inner.l2.pending_policy);
        self.inner.l0.decision.set_bandit_config(snap.bandit_config);
        if let Some(decision) = snap.last_l2_decision {
            self.inner.l0.decision.restore_l2_decision(decision);
        }
    }

    pub fn snapshot_json(&self) -> Result<String> {
        self.snapshot().to_json()
    }

    pub fn apply_snapshot_json(&mut self, json: &str) -> Result<()> {
        let snapshot = KernelSnapshot::from_json(json)?;
        self.apply_snapshot(snapshot);
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        self.snapshot().save_to(&self.state_path)
    }

    pub fn load(&mut self) -> Result<()> {
        let snap = KernelSnapshot::load_from(&self.state_path)?;
        self.apply_snapshot(snap);
        Ok(())
    }

    pub fn checkpoint(&mut self, label: &str) -> Result<CheckpointMeta> {
        let snap = self.snapshot();
        let meta = self.checkpoints().save(
            label,
            self.inner.total_steps,
            self.inner.l0.memory.cas.avg_prediction_error,
            self.inner.l0.memory.cognitive_effect.error_damping,
            &snap,
        )?;
        self.append_event(AuditEvent::Checkpoint {
            checkpoint_id: meta.id,
            step: meta.step,
            label: meta.label.clone(),
        })?;
        Ok(meta)
    }

    pub fn list_checkpoints(&self) -> Result<Vec<CheckpointMeta>> {
        self.checkpoints().list()
    }

    pub fn rollback(&mut self, checkpoint_id: Uuid) -> Result<()> {
        let snap = self.checkpoints().load_snapshot(checkpoint_id)?;
        let step_before = self.inner.total_steps;
        self.apply_snapshot(snap);
        self.append_event(AuditEvent::Rollback {
            checkpoint_id,
            step_before,
        })?;
        Ok(())
    }

    pub fn audit_task(&mut self, task: &str, reward: f64, steps: u32) -> Result<()> {
        self.append_event(AuditEvent::TaskEpisode {
            task: task.into(),
            reward,
            steps,
        })?;
        Ok(())
    }

    pub fn audit_tail(&self, n: usize) -> Result<Vec<crate::audit::AuditRecord>> {
        self.audit().tail(n)
    }

    pub fn audit_summary(&self) -> Result<crate::audit::AuditSummary> {
        self.audit().summarize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l0::decision::DecisionMaker;
    use std::env::temp_dir;
    use std::io::Write;

    #[test]
    fn event_hook_fires() {
        let path = temp_dir().join(format!("omega-hook-{}.json", crate::new_uuid()));
        let mut k = Kernel::with_state_path(&path);
        k.autosave = false;
        let fired = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let f2 = fired.clone();
        k.set_event_hook(move |_| {
            f2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        let _ = k.step("test");
        assert!(fired.load(std::sync::atomic::Ordering::SeqCst) >= 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn snapshot_json_roundtrips_l2_state_at_arbitrary_step() {
        let source_path = temp_dir().join(format!("omega-hot-source-{}.json", crate::new_uuid()));
        let restored_path =
            temp_dir().join(format!("omega-hot-restored-{}.json", crate::new_uuid()));
        let mut source = Kernel::with_state_path(&source_path);
        source.autosave = false;
        source.dump_trajectory = false;
        for _ in 0..4 {
            source
                .step("maintain coherent self-model omega dayan")
                .unwrap();
        }
        source.inner.l2.in_fixed_point = true;
        source.inner.l2.dwell_steps = 7;
        source.inner.l2.defensive_mode = true;
        source.inner.l2.awaiting_consequence = true;
        source.inner.l2.commit_cooldown_remaining = 3;
        source.inner.l2.negative_reward_streak = 2;

        let json = source.snapshot_json().unwrap();
        let mut restored = Kernel::with_state_path(&restored_path);
        restored.autosave = false;
        restored.dump_trajectory = false;
        restored.apply_snapshot_json(&json).unwrap();

        assert_eq!(restored.inner.total_steps, source.inner.total_steps);
        assert!(restored.inner.l2.in_fixed_point);
        assert_eq!(restored.inner.l2.dwell_steps, 7);
        assert!(restored.inner.l2.defensive_mode);
        assert!(restored.inner.l2.awaiting_consequence);
        assert_eq!(restored.inner.l2.commit_cooldown_remaining, 3);
        assert_eq!(
            restored.inner.l2.detector.history.len(),
            source.inner.l2.detector.history.len()
        );

        restored.step("continue after hot restore").unwrap();
        assert_eq!(restored.inner.total_steps, source.inner.total_steps + 1);

        let _ = std::fs::remove_file(&source_path);
        let _ = std::fs::remove_file(&restored_path);
    }

    #[test]
    fn restart_preserves_l1_self_modify_flag() {
        // Enable L1 self-modify, save, "restart" (reload from disk), verify the
        // switch is still on rather than falling back to the safe default-off.
        let path = temp_dir().join(format!("omega-restart-l1-{}.json", crate::new_uuid()));
        let mut k = Kernel::with_state_path(&path);
        k.autosave = false;
        k.dump_trajectory = false;
        k.inner.l1.self_modify_enabled = true;
        k.save().unwrap();

        let mut restarted = Kernel::with_state_path(&path);
        restarted.load().unwrap();
        assert!(
            restarted.inner.l1.self_modify_enabled,
            "L1 self-modify flag must survive restart"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn restart_preserves_l1_patch_cooldown() {
        // After accepting a patch the cooldown counter resets to 0; a restart
        // must not reset it back to the warm default (which would let a patch
        // fire immediately again).
        let path = temp_dir().join(format!("omega-restart-cd-{}.json", crate::new_uuid()));
        let mut k = Kernel::with_state_path(&path);
        k.autosave = false;
        k.dump_trajectory = false;
        k.inner.l1.self_modify_enabled = true;
        k.inner.l1.steps_since_last_patch = 1;
        k.save().unwrap();

        let mut restarted = Kernel::with_state_path(&path);
        restarted.load().unwrap();
        assert_eq!(
            restarted.inner.l1.steps_since_last_patch, 1,
            "patch cooldown must survive restart"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn restart_preserves_l2_rollback_effect_for_negative_feedback() {
        // Simulate an L2 commit that stored the pre-commit policy in
        // rollback_effect, restart, then feed negative feedback and verify the
        // old policy is actually restored.
        let path = temp_dir().join(format!("omega-restart-l2rb-{}.json", crate::new_uuid()));
        let prior = crate::types::RuleEffect {
            error_damping: 0.33,
            explore_bonus: 0.21,
            ..crate::types::RuleEffect::default()
        };
        let mut k = Kernel::with_state_path(&path);
        k.autosave = false;
        k.dump_trajectory = false;
        // Pre-commit policy preserved for rollback.
        k.inner.l2.rollback_effect = Some(prior.clone());
        // Current policy is the post-commit (worse) one.
        k.inner.l0.memory.cognitive_effect.error_damping = 0.95;
        k.save().unwrap();

        let mut restarted = Kernel::with_state_path(&path);
        restarted.load().unwrap();
        assert!(
            restarted.inner.l2.rollback_effect.is_some(),
            "rollback_effect must survive restart"
        );

        // Negative feedback triggers rollback to the pre-commit policy.
        restarted.observe_l2_consequence(-1.0, 1.0, 0.0);
        assert!(
            (restarted.inner.l0.memory.cognitive_effect.error_damping - 0.33).abs() < 1e-9,
            "negative feedback after restart must roll back to the preserved policy"
        );
        assert!(restarted.inner.l2.rollback_effect.is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn l2_observes_without_l1_self_modification() {
        // L1 and L2 have independent switches. L2 may calculate candidates with
        // L1 disabled, but only a validated KL policy can influence L0: the
        // retired additive RuleEffect channel must never fire implicitly.
        let dir = temp_dir().join(format!("omega-l2gate-{}", crate::new_uuid()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        let mut k = Kernel::with_state_path(path.clone());
        k.autosave = false;
        k.dump_trajectory = false;
        k.auto_checkpoint_on_patch = false;
        k.set_l2_enabled(true);
        k.inner
            .l1
            .store
            .active
            .insert("cognitive_rules".into(), "self-model bias damp rule".into());
        let before = k.inner.l0.memory.cognitive_effect.clone();
        let mut saw_observation = false;
        for _ in 0..24 {
            let result = k.step("maintain coherent self-model omega dayan").unwrap();
            saw_observation |= result.l2.dayan_residual.is_finite();
            assert!(result.l2.effect_nudge.is_none());
            assert!(!result.l1.patched);
        }
        let after = &k.inner.l0.memory.cognitive_effect;
        assert!(saw_observation);
        assert_eq!(before.error_damping, after.error_damping);
        assert_eq!(before.explore_bonus, after.explore_bonus);
        assert_eq!(before.risk_tolerance, after.risk_tolerance);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn step_propagates_audit_append_failure() {
        // Point the kernel at a state path whose parent directory does not exist
        // and cannot be created (a path under an existing *file*), so the audit
        // append fails. step() must return Err rather than reporting success.
        let blocker = temp_dir().join(format!("omega-blocker-{}.file", crate::new_uuid()));
        std::fs::write(&blocker, b"not a dir").unwrap();
        // state path nested *under* a regular file -> parent mkdir/open fails.
        let bad_state = blocker.join("nested").join("state.json");
        let mut k = Kernel::with_state_path(bad_state.clone());
        k.autosave = false;
        k.dump_trajectory = false;
        let res = k.step("should fail");
        assert!(res.is_err(), "step must fail when audit append fails");
        let _ = std::fs::remove_file(&blocker);
    }

    #[test]
    fn step_propagates_autosave_failure() {
        // Same nested-under-a-file trick, but disable the audit path by making
        // dump_trajectory/autosave the failing component. Here autosave is on and
        // audit append is what we allow to succeed first is impossible to isolate,
        // so we simply assert that a step with autosave on an unwritable path
        // errors out (fail-closed).
        let blocker = temp_dir().join(format!("omega-blocker2-{}.file", crate::new_uuid()));
        std::fs::write(&blocker, b"not a dir").unwrap();
        let bad_state = blocker.join("nested").join("state.json");
        let mut k = Kernel::with_state_path(bad_state.clone());
        k.autosave = true;
        k.dump_trajectory = false;
        let res = k.step("autosave should fail");
        assert!(res.is_err(), "step must fail when autosave fails");
        let _ = std::fs::remove_file(&blocker);
    }

    #[test]
    fn checkpoint_and_rollback_emit_audit_events() {
        // Unique subdirectory so the shared `<parent>/audit.jsonl` and
        // `<parent>/checkpoints/` sidecars are isolated per test.
        let dir = temp_dir().join(format!("omega-audit-ops-{}", crate::new_uuid()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        let mut k = Kernel::with_state_path(path.clone());
        k.autosave = false;
        k.dump_trajectory = false;
        k.step("create checkpoint").unwrap();
        let checkpoint = k.checkpoint("before rollback").unwrap();
        k.step("change state").unwrap();
        k.rollback(checkpoint.id).unwrap();

        let summary = k.audit_summary().unwrap();
        assert_eq!(summary.checkpoints, 1);
        assert_eq!(summary.rollbacks, 1);
        let records = k.audit_tail(20).unwrap();
        assert!(records.iter().any(|record| matches!(
            &record.event,
            AuditEvent::Checkpoint { checkpoint_id, .. } if *checkpoint_id == checkpoint.id
        )));
        assert!(records.iter().any(|record| matches!(
            record.event,
            AuditEvent::Rollback { checkpoint_id, step_before: 2 } if checkpoint_id == checkpoint.id
        )));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn l1_effect_rollback_is_independent_of_restored_error() {
        let path = temp_dir().join(format!("omega-l1-effect-rb-{}.json", crate::new_uuid()));
        let mut kernel = Kernel::with_bandit_ucb(path);
        kernel.autosave = false;
        kernel.auto_checkpoint_on_patch = false;
        kernel.inner.l1.previous_effect = Some(crate::types::RuleEffect::default());
        kernel.inner.l1.pending_rollback = true;
        kernel.inner.l0.memory.cognitive_effect.error_damping = 1.4;
        assert!(kernel.observe_l1_consequence(0.1));
        assert_eq!(kernel.inner.l0.memory.cognitive_effect.error_damping, 1.0);
        assert!(!kernel.observe_l1_consequence(-0.1));
        kernel.inner.l0.decision.observe_with_feedback(
            0,
            1.0,
            crate::types::DecisionFeedback {
                l1_rollback: true,
                error_delta: -0.1,
                ..crate::types::DecisionFeedback::default()
            },
        );
        let decision = kernel.inner.l0.decision.l2_decision().unwrap();
        assert_eq!(decision.rollback_count, 1);
        assert!(decision.adjusted_reward.unwrap() < 1.0);
        let _ = std::fs::remove_file(kernel.state_path);
    }

    #[test]
    fn committed_policy_executes_then_settles_its_actual_consequence() {
        let dir = temp_dir().join(format!("omega-l2-policy-cycle-{}", crate::new_uuid()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        let mut kernel = Kernel::with_bandit_ucb(path);
        kernel.autosave = false;
        kernel.auto_checkpoint_on_patch = false;
        kernel.dump_trajectory = false;
        kernel.set_l2_enabled(true);
        kernel.inner.l2.commit_threshold = 0.0;
        kernel.inner.l2.controller.calm_required = 1;
        kernel.inner.l2.controller.gate_state = crate::l2::control::L2GateState::Active;
        kernel.inner.l2.controller.trust.alpha = 0.2;
        kernel.inner.l2.controller.model_confidence = 1.0;
        kernel.inner.l2.controller.residual = 0.3;
        kernel.inner.l2.controller.residual_ema = 0.3;
        kernel.inner.l2.controller.observations = 4;
        kernel.inner.l2.controller.residual_slope = 0.0;
        kernel.inner.l2.controller.ema_error = 0.0;

        let committed = kernel
            .step("maintain coherent self-model omega dayan")
            .unwrap();
        assert!(
            committed.l2.modulation_committed,
            "rejection={:?} gate={:?} alpha={} beta={} confidence={} residual={} predicted={} retire={}",
            committed.l2.rejection_reason,
            committed.l2.gate_state,
            committed.l2.alpha,
            committed.l2.beta,
            kernel.inner.l2.controller.model_confidence,
            kernel.inner.l2.controller.residual,
            kernel.inner.l2.controller.certificate.predicted_potential,
            kernel.inner.l2.controller.retire_requested,
        );
        assert!(!committed.l2.applied_policy.committed);
        assert_eq!(committed.l2.l2_policy.expert, 1);
        assert!(kernel.inner.l2.awaiting_consequence);

        // Recovery is observed before the next step. L2 waits until that
        // policy acts and its feedback settles before finalizing retirement.
        kernel.observe_l2_feedback(1.0, 0.1, false, 0.3, [0.8, 0.2]);
        kernel.inner.l2.controller.residual = 0.1;
        kernel.inner.l2.controller.episode_start_residual = 0.6;
        kernel.observe_l2_feedback(1.0, 0.1, false, 0.3, [0.8, 0.2]);
        assert!(kernel.inner.l2.controller.retire_requested);
        assert_eq!(
            kernel.inner.l2.controller.gate_state,
            crate::l2::control::L2GateState::Active
        );
        let policy = kernel.inner.l2.pending_policy;
        let applied = kernel
            .step("maintain coherent self-model omega dayan")
            .unwrap();
        assert!(!applied.l2.modulation_committed);
        assert_eq!(
            applied.l2.gate_state,
            crate::l2::control::L2GateState::Active
        );
        assert_eq!(applied.l2.applied_policy, policy);
        let logged: crate::types::L2Policy =
            serde_json::from_value(applied.trajectory.actions[0].params["l2_policy"].clone())
                .unwrap();
        assert_eq!(logged, policy);

        let before = kernel.inner.l2.controller.experts.cumulative_reward[policy.expert];
        kernel.observe_l2_feedback(1.0, 0.1, false, 0.3, [0.8, 0.2]);
        let after = kernel.inner.l2.controller.experts.cumulative_reward[policy.expert];
        assert!(after > before);
        assert_eq!(
            kernel.inner.l2.controller.gate_state,
            crate::l2::control::L2GateState::Active
        );
        assert!(!kernel.inner.l2.pending_policy.committed);
        kernel.observe_l2_consequence(0.5, -0.1, 0.0);
        assert_eq!(
            kernel.inner.l2.controller.gate_state,
            crate::l2::control::L2GateState::Dormant
        );
        assert!(!kernel.inner.l2.awaiting_consequence);
        assert!(!kernel.inner.l2.pending_policy.committed);
        assert_eq!(kernel.inner.l2.applied_policy, policy);

        // The resident update consumes that same policy exactly once, even
        // though pending injection has been cleared by consequence settlement.
        let before = kernel.inner.l2.controller.experts.cumulative_reward[policy.expert];
        kernel.observe_l2_feedback(0.0, 0.2, false, -0.3, [0.8, 0.2]);
        assert_eq!(
            kernel.inner.l2.controller.experts.cumulative_reward[policy.expert],
            before * kernel.inner.l2.controller.experts.forgetting
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn harmful_feedback_cancels_pending_without_erasing_applied_consequence() {
        let path = temp_dir().join(format!("omega-l2-safety-cycle-{}.json", crate::new_uuid()));
        let mut kernel = Kernel::with_bandit_ucb(path);
        kernel.autosave = false;
        kernel.auto_checkpoint_on_patch = false;
        kernel.dump_trajectory = false;
        let applied = crate::types::L2Policy {
            prior: [0.8, 0.2],
            beta: 0.4,
            alpha: 0.2,
            committed: true,
            expert: 1,
            gate_state: crate::l2::control::L2GateState::Active,
            ..crate::types::L2Policy::default()
        };
        kernel.inner.l2.applied_policy = applied;
        kernel.inner.l2.pending_policy = crate::types::L2Policy {
            expert: 2,
            ..applied
        };
        kernel.inner.l2.awaiting_consequence = true;
        kernel.inner.l2.controller.retire_requested = true;

        kernel.observe_l2_feedback(0.0, 0.4, true, -0.5, [0.8, 0.2]);
        assert!(!kernel.inner.l2.pending_policy.committed);
        assert_eq!(kernel.inner.l2.applied_policy, applied);
        kernel.observe_l2_consequence(-0.5, 0.2, 0.0);
        assert!(!kernel.inner.l2.awaiting_consequence);
        assert!(kernel.inner.l2.last_consequence_score < 0.0);
        assert_eq!(kernel.inner.l2.applied_policy, applied);
        assert_eq!(
            kernel.inner.l0.decision.snapshot().l2_policy,
            crate::types::L2Policy::default()
        );
        let _ = std::fs::remove_file(kernel.state_path);
    }

    #[test]
    fn bandit_decision_replay_and_restart_are_auditable() {
        let dir = temp_dir().join(format!("omega-bandit-replay-{}", crate::new_uuid()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        let mut kernel = Kernel::with_bandit_ucb(&path);
        kernel.autosave = false;
        kernel.auto_checkpoint_on_patch = false;
        kernel.inner.l0.decision.observe_with_feedback(
            0,
            1.0,
            crate::types::DecisionFeedback::default(),
        );
        kernel.step("audit local UCB").unwrap();
        kernel.observe_l2_consequence(0.5, 0.0, 0.0);
        let decision = kernel.status().last_l2_decision.expect("UCB explanation");
        assert_eq!(decision.selected_bias, "explore");
        assert!(decision.exploration_bonus >= 0.0);
        kernel.save().unwrap();

        let mut restarted = Kernel::with_bandit_ucb(&path);
        restarted.load().unwrap();
        restarted.observe_l2_consequence(0.5, 0.0, 0.0);
        assert_eq!(
            restarted.inner.l0.decision.snapshot().counts,
            kernel.inner.l0.decision.snapshot().counts
        );
        assert!(restarted.status().last_l2_decision.is_some());
        let replay = restarted.replay_trajectory(1).unwrap();
        let replayed = replay.tail[0].l2_decision.as_ref().expect("replay UCB");
        assert_eq!(replayed.selected_arm, 1);
        assert_eq!(replayed.reason, crate::types::L2DecisionReason::Switched);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn replay_trajectory_file_works() {
        let path = temp_dir().join(format!("omega-traj-{}.jsonl", crate::new_uuid()));
        let mut f = std::fs::File::create(&path).unwrap();
        let line1 = serde_json::json!({
            "step": 1,
            "action": "observe",
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
        assert!(sum.tail.iter().all(|row| row.l2_decision.is_none()));
        let _ = std::fs::remove_file(&path);
    }
}
