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

/// In-process event callback (after audit append).
pub type EventHook = Box<dyn FnMut(&AuditEvent) + Send>;

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
    /// Optional in-process hook invoked after each audit event is appended.
    event_hook: Option<EventHook>,
}
