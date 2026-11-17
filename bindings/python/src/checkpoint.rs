use pyo3::prelude::*;
use crate::types::CheckpointMeta;
use crate::error::{PythonBindingError, PyResult};
use std::path::Path;

/// Python wrapper for CheckpointStore
#[pyclass]
pub struct CheckpointStore {
    dir: String,
    max_keep: usize,
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
            max_keep: 32,
        })
    }

    /// List all checkpoints (newest first)
    pub fn list(&self) -> PyResult<Vec<CheckpointMeta>> {
        use std::fs;
        if !Path::new(&self.dir).exists() {
            return Ok(vec![]);
        }
        let mut out = Vec::new();
        for entry in fs::read_dir(&self.dir).map_err(PythonBindingError::Io)? {
            let entry = entry.map_err(PythonBindingError::Io)?;
            let path = entry.path();
            if !path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with(".meta.json"))
                .unwrap_or(false)
            {
                continue;
            }
            let data = fs::read_to_string(&path).map_err(PythonBindingError::Io)?;
            if let Ok(m) = serde_json::from_str::<CheckpointMeta>(&data) {
                out.push(m);
            }
        }
        out.sort_by_key(|a| std::cmp::Reverse(a.created_at.clone()));
        Ok(out)
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