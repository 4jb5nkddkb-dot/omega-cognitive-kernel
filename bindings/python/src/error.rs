use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PythonBindingError {
    #[error("Kernel error: {0}")]
    Kernel(#[from] omega_cognitive_kernel::OmegaError),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("UUID error: {0}")]
    Uuid(#[from] uuid::Error),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("{0}")]
    SnapshotJson(String),
    #[error("Checkpoint not found: {0}")]
    CheckpointNotFound(String),
}

impl From<PythonBindingError> for PyErr {
    fn from(err: PythonBindingError) -> PyErr {
        match err {
            PythonBindingError::Serde(_)
            | PythonBindingError::InvalidArgument(_)
            | PythonBindingError::SnapshotJson(_) => PyValueError::new_err(err.to_string()),
            PythonBindingError::Kernel(omega_cognitive_kernel::OmegaError::Persist(_)) => {
                PyValueError::new_err(err.to_string())
            }
            _ => PyRuntimeError::new_err(err.to_string()),
        }
    }
}

pub type PyResult<T> = Result<T, PythonBindingError>;
