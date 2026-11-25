use crate::error::{PyResult, PythonBindingError};
use crate::types::{AuditRecord, AuditSummary};
use pyo3::prelude::*;
use std::path::Path;

/// Python wrapper for AuditLog
#[pyclass]
pub struct AuditLog {
    path: String,
}

#[pymethods]
impl AuditLog {
    /// Create an AuditLog from a state file path
    #[staticmethod]
    pub fn beside_state(state_path: &str) -> PyResult<Self> {
        let path = Path::new(state_path)
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("audit.jsonl");
        Ok(Self {
            path: path.to_string_lossy().to_string(),
        })
    }

    /// Read all audit records
    pub fn read_all(&self) -> PyResult<Vec<AuditRecord>> {
        let log = omega_cognitive_kernel::AuditLog {
            path: Path::new(&self.path).to_path_buf(),
        };
        log.read_all()
            .map(|rows| rows.into_iter().map(Into::into).collect())
            .map_err(PythonBindingError::Kernel)
            .map_err(Into::into)
    }

    /// Get the last n audit records
    pub fn tail(&self, n: usize) -> PyResult<Vec<AuditRecord>> {
        let mut rows = self.read_all()?;
        if rows.len() > n {
            rows = rows.split_off(rows.len() - n);
        }
        Ok(rows)
    }

    /// Get current file length in bytes (for polling)
    pub fn byte_len(&self) -> PyResult<u64> {
        use std::fs;
        if !Path::new(&self.path).exists() {
            return Ok(0);
        }
        let meta = fs::metadata(&self.path).map_err(PythonBindingError::Io)?;
        Ok(meta.len())
    }

    /// Poll for new records after offset
    pub fn poll_after(&self, offset: u64) -> PyResult<(Vec<AuditRecord>, u64)> {
        let log = omega_cognitive_kernel::AuditLog {
            path: Path::new(&self.path).to_path_buf(),
        };
        log.poll_after(offset)
            .map(|(rows, next)| (rows.into_iter().map(Into::into).collect(), next))
            .map_err(PythonBindingError::Kernel)
            .map_err(Into::into)
    }

    /// Generate summary statistics
    pub fn summarize(&self) -> PyResult<AuditSummary> {
        let log = omega_cognitive_kernel::AuditLog {
            path: Path::new(&self.path).to_path_buf(),
        };
        let summary = log.summarize().map_err(PythonBindingError::Kernel)?;
        let value = serde_json::to_value(summary).map_err(PythonBindingError::Serde)?;
        serde_json::from_value(value).map_err(PythonBindingError::Serde)
    }
}
