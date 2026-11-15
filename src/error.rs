#[allow(unused_imports)]
use crate::prelude::*;

use alloc::string::String;
use core::fmt;

#[derive(Debug)]
pub enum OmegaError {
    ConstraintViolation(String),
    SandboxRejection(String),
    ExecutionBlocked(String),
    Memory(String),
    Persist(String),
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
