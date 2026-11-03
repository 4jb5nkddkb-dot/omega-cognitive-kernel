use thiserror::Error;

#[derive(Error, Debug)]
pub enum OmegaError {
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    #[error("Sandbox rejection: {0}")]
    SandboxRejection(String),

    #[error("Execution blocked: {0}")]
    ExecutionBlocked(String),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("Persistence error: {0}")]
    Persist(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, OmegaError>;
