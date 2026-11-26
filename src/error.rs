#[allow(unused_imports)]
use crate::prelude::*;

use alloc::string::String;
use core::fmt;
use uuid::Uuid;

#[derive(Debug)]
pub enum OmegaError {
    ConstraintViolation(String),
    SandboxRejection(String),
    ExecutionBlocked(String),
    Memory(String),
    Persist(String),
    StepNotExecuted { step_id: Uuid, reason: String },
    StepExecutionOutcomeUnknown { step_id: Uuid, reason: String },
    StepCommitIncomplete { step_id: Uuid, stage: String },
    StepAlreadyCommitted { step_id: Uuid },
    StepIdConflict { step_id: Uuid },
    Config(String),
    Other(anyhow::Error),
}

impl fmt::Display for OmegaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConstraintViolation(value) => write!(f, "Constraint violation: {value}"),
            Self::SandboxRejection(value) => write!(f, "Sandbox rejection: {value}"),
            Self::ExecutionBlocked(value) => write!(f, "Execution blocked: {value}"),
            Self::Memory(value) => write!(f, "Memory error: {value}"),
            Self::Persist(value) => write!(f, "Persistence error: {value}"),
            Self::StepNotExecuted { step_id, reason } => {
                write!(f, "Step {step_id} was not executed: {reason}")
            }
            Self::StepExecutionOutcomeUnknown { step_id, reason } => write!(
                f,
                "Step {step_id} execution outcome is unknown: {reason}; external effects may have occurred"
            ),
            Self::StepCommitIncomplete { step_id, stage } => write!(
                f,
                "Step {step_id} executed but commit is incomplete at {stage}; do not retry automatically"
            ),
            Self::StepAlreadyCommitted { step_id } => write!(
                f,
                "Step {step_id} is already committed; duplicate was not executed"
            ),
            Self::StepIdConflict { step_id } => write!(
                f,
                "Step ID {step_id} was already used with a different goal"
            ),
            Self::Config(value) => write!(f, "Config error: {value}"),
            Self::Other(error) => error.fmt(f),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OmegaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Other(error) => error.source(),
            _ => None,
        }
    }
}

impl From<anyhow::Error> for OmegaError {
    fn from(error: anyhow::Error) -> Self {
        Self::Other(error)
    }
}

pub type Result<T> = core::result::Result<T, OmegaError>;
