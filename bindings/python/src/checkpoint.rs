use crate::error::{PyResult, PythonBindingError};
use crate::types::CheckpointMeta;
use pyo3::prelude::*;
use std::path::Path;

/// Python wrapper for CheckpointStore
#[pyclass]
pub struct CheckpointStore {
    dir: String,
}

#[pymethods]
impl CheckpointStore {
    /// Create a CheckpointStore from a state file path
    #[staticmethod]
    pub fn beside_state(state_path: &str) -> PyResult<Self> {
        let dir = Path::new(state_path)
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("checkpoints");
        Ok(Self {
            dir: dir.to_string_lossy().to_string(),
        })
    }

    /// List all checkpoints (newest first)
    pub fn list(&self) -> PyResult<Vec<CheckpointMeta>> {
        let store = omega_cognitive_kernel::checkpoint::CheckpointStore {
            dir: Path::new(&self.dir).to_path_buf(),
            max_keep: 32,
        };
        let checkpoints = store.list().map_err(PythonBindingError::Kernel)?;
        checkpoints
            .into_iter()
            .map(|checkpoint| {
                serde_json::to_value(checkpoint)
                    .map_err(PythonBindingError::Serde)
                    .and_then(|value| {
                        serde_json::from_value(value).map_err(PythonBindingError::Serde)
                    })
            })
            .collect()
    }

    /// Load a snapshot by checkpoint ID
    pub fn load_snapshot(&self, checkpoint_id: &str) -> PyResult<String> {
        use std::fs;
        let uuid = uuid::Uuid::parse_str(checkpoint_id).map_err(PythonBindingError::Uuid)?;
        let snap_path = Path::new(&self.dir).join(format!("{uuid}.state.json"));
        let data = fs::read_to_string(&snap_path).map_err(PythonBindingError::Io)?;
        Ok(data)
    }
}
