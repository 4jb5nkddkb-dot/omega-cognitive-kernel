#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use core::sync::atomic::{AtomicU32, Ordering};

static UUID_SEQUENCE: AtomicU32 = AtomicU32::new(1);

/// Generates process-local identifiers without requiring an OS random source.
/// Host applications can replace this with an external identity source when
/// globally unique IDs are required; the core only needs stable instance IDs.
pub fn new_uuid() -> uuid::Uuid {
    let sequence = UUID_SEQUENCE.fetch_add(1, Ordering::Relaxed) as u128;
    uuid::Uuid::from_u128(0x4f4d_4547_412d_4b45_524e_454c_0000_0000u128 | sequence)
}

#[cfg(feature = "std")]
pub fn now_utc() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

#[cfg(not(feature = "std"))]
pub fn now_utc() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::<chrono::Utc>::UNIX_EPOCH
}

#[cfg(feature = "std")]
pub fn now_utc_string() -> alloc::string::String {
    now_utc().to_rfc3339()
}

#[cfg(not(feature = "std"))]
pub fn now_utc_string() -> alloc::string::String {
    "1970-01-01T00:00:00+00:00".into()
}

#[doc(hidden)]
pub mod prelude {
    pub use alloc::{
        boxed::Box,
        format,
        string::{String, ToString},
        vec,
        vec::Vec,
    };
}

#[cfg(feature = "std")]
pub mod api;
#[cfg(feature = "std")]
pub mod audit;
#[cfg(feature = "std")]
pub mod checkpoint;
#[cfg(feature = "std")]
pub mod config;
pub mod error;
pub mod l0;
pub mod l1;
pub mod l2;
pub mod memory;
pub mod orchestrator;
#[cfg(feature = "std")]
pub mod persist;
pub mod safety;
#[cfg(feature = "std")]
pub mod task;
pub mod types;

#[cfg(feature = "std")]
pub use api::{
    replay_trajectory_file, Kernel, KernelStatus, RunReport, TrajectoryReplayRow,
    TrajectoryReplaySummary,
};
#[cfg(feature = "std")]
pub use audit::{AuditEvent, AuditLog, AuditRecord, AuditSummary, KernelEvent};
#[cfg(feature = "std")]
pub use config::{InterventionLevel, InterventionMode, InterventionPolicy, KernelConfig};
pub use error::{OmegaError, Result};
pub use orchestrator::{OmegaOrchestrator, StepResult};
#[cfg(feature = "std")]
pub use persist::{KernelSnapshot, CURRENT_SCHEMA_VERSION, DEFAULT_STATE_PATH};
#[cfg(feature = "std")]
pub use task::{
    run_bandit_task, run_l2_benchmark, run_l2_benchmark_scenario,
    run_l2_benchmark_scenario_with_gate, BanditReport, L2BenchmarkAggregate, L2BenchmarkReport,
};
pub use types::*;

// Public adapter surface — implement these to plug real world I/O into the kernel.
pub use l0::decision::{ConstrainedDecisionMaker, DecisionMaker};
#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
pub use l0::executor::{AsyncExecutor, AsyncExecutorAdapter, AsyncLoggingExecutor};
pub use l0::executor::{Executor, SandboxedExecutor};
#[cfg(feature = "std")]
pub use l0::executor::{LoggingExecutor, ToolGateExecutor};
#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
pub use l0::observer::{
    AsyncDirSnapshotObserver, AsyncFileObserver, AsyncHttpObserver, AsyncObserver,
    AsyncObserverAdapter,
};
#[cfg(feature = "std")]
pub use l0::observer::{DirSnapshotObserver, FileObserver, HttpObserver};
pub use l0::observer::{MultiModalObserver, Observer};
pub use l0::predictor::{ModelBasedPredictor, Predictor};
