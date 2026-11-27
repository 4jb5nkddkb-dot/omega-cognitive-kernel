#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "std", doc = include_str!("../README.md"))]

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
pub mod body;
#[cfg(feature = "std")]
pub mod checkpoint;
#[cfg(feature = "std")]
pub mod config;
#[cfg(all(feature = "std", feature = "toml"))]
pub mod config_toml;
pub mod critic;
pub mod error;
pub mod goals;
pub mod introspect;
pub mod l0;
pub mod l1;
pub mod l2;
pub mod llm;
pub mod memory;
pub mod orchestrator;
#[cfg(feature = "std")]
pub mod persist;
pub mod planning;
#[cfg(feature = "std")]
mod private_fs;
pub mod reactor;
pub mod safety;
#[cfg(feature = "std")]
pub mod task;
pub mod types;

#[cfg(feature = "std")]
pub use api::{
    replay_trajectory_file, Kernel, KernelStatus, RunReport, TrajectoryReplayRow,
    TrajectoryReplaySummary, DEFAULT_MAX_TRAJECTORY_BYTES,
};
#[cfg(feature = "std")]
pub use audit::{AuditEvent, AuditLog, AuditRecord, AuditSummary, KernelEvent};
#[cfg(feature = "std")]
pub use config::{
    InterventionLevel, InterventionMode, InterventionPolicy, KernelConfig, L2Config, PrivacyConfig,
};
pub use critic::{Critic, ReflectionReport, RewardSignal, TrialBudget};
pub use error::{OmegaError, Result};
pub use orchestrator::{OmegaOrchestrator, StepResult};
#[cfg(feature = "std")]
pub use persist::{KernelSnapshot, CURRENT_SCHEMA_VERSION, DEFAULT_STATE_PATH};
#[cfg(feature = "std")]
pub use task::{
    run_bandit_task, run_l1_l2_factorial_benchmark, run_l2_benchmark, run_l2_benchmark_scenario,
    run_l2_benchmark_scenario_with_gate, run_l2_gain_matrix, BanditReport, FactorialArm,
    FactorialBenchmarkReport, FactorialEffect, FactorialRun, L2BenchmarkAggregate,
    L2BenchmarkReport, L2GainAssessment, L2GainMatrixReport, L2GainScenarioAssessment,
    L2GainSeedSet, L2PairedRun, PairedMeanDifference, L2_GAIN_DEFAULT_EPISODES,
    L2_GAIN_DEFAULT_RUNS, L2_GAIN_DEFAULT_STEPS, L2_GAIN_DISCOVERY_SEED, L2_GAIN_SCENARIOS,
    L2_GAIN_VALIDATION_SEED,
};
pub use types::*;

// Public adapter surface — implement these to plug real world I/O into the kernel.
pub use body::{
    BodyBus, BodyChannelInfo, BodyCommandProposal, BodyPacket, BodyPacketStatus, BodySummary,
    ChannelKind,
};
pub use goals::{
    GoalNode, GoalStatus, GoalTree, GoalTreeSummary, IntrinsicMotivation, IntrinsicSignal,
};
pub use introspect::{EntropyReport, EntropyWatchdog};
pub use l0::bandit_decision::{BanditUcbDecisionMaker, BanditUcbSnapshot};
pub use l0::decision::{ConstrainedDecisionMaker, DecisionMaker};
#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
pub use l0::executor::{AsyncExecutor, AsyncExecutorAdapter, AsyncLoggingExecutor};
pub use l0::executor::{Executor, SandboxedExecutor};
#[cfg(feature = "std")]
pub use l0::executor::{LoggingExecutor, ToolGateExecutor};
pub use l0::hybrid_predictor::{HybridWorldModel, HybridWorldPredictor, TinyAttentionPredictor};
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
pub use l1::{L1SpectralReflector, SpectralBin, SpectralPatch};
pub use l2::{
    CognitiveRealm, KernelStepOutput, L2CognitiveGate, L2Decision, L2State, SpectralGateConfig,
};
pub use llm::{parse_suggestion_json, LlmBridge, LlmProposal, SchemaResult};
pub use memory::{
    export_weights, migrate_payload, squash_delta_history, verify_weights, WeightsPayload,
    WEIGHTS_MAGIC, WEIGHTS_VERSION,
};
pub use planning::{Plan, PlanStep, ShortHorizonPlanner};
pub use reactor::{PriorityReactor, ReactorTask};
pub use safety::{ActionBoundary, AuthorizationToken, TokenBook};
#[cfg(feature = "std")]
pub use safety::{StorageLock, Vault};
