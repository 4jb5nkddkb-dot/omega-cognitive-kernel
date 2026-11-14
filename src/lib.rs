pub mod api;
pub mod audit;
pub mod checkpoint;
pub mod config;
pub mod error;
pub mod l0;
pub mod l1;
pub mod l2;
pub mod memory;
pub mod orchestrator;
pub mod persist;
pub mod safety;
pub mod task;
pub mod types;

pub use api::{
    replay_trajectory_file, Kernel, KernelStatus, RunReport, TrajectoryReplayRow,
    TrajectoryReplaySummary,
};
pub use audit::{AuditEvent, AuditLog, AuditRecord, AuditSummary, KernelEvent};
pub use config::{InterventionLevel, InterventionMode, InterventionPolicy, KernelConfig};
pub use error::{OmegaError, Result};
pub use orchestrator::{OmegaOrchestrator, StepResult};
pub use persist::{KernelSnapshot, CURRENT_SCHEMA_VERSION, DEFAULT_STATE_PATH};
pub use task::{run_bandit_task, BanditReport};
pub use types::*;

// Public adapter surface — implement these to plug real world I/O into the kernel.
pub use l0::decision::{ConstrainedDecisionMaker, DecisionMaker};
pub use l0::executor::{Executor, LoggingExecutor, SandboxedExecutor, ToolGateExecutor};
pub use l0::observer::{
    DirSnapshotObserver, FileObserver, HttpObserver, MultiModalObserver, Observer,
};
pub use l0::predictor::{ModelBasedPredictor, Predictor};
