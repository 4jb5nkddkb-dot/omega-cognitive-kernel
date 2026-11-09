use crate::error::{OmegaError, Result};
use crate::types::{Modality, Observation};
use chrono::Utc;
use std::path::{Path, PathBuf};

/// External world observation interface.
///
/// Implement this trait to feed real sensors, files, HTTP status, or any other
/// source into the kernel. The default `MultiModalObserver` is a pure simulation.
pub trait Observer {
    fn observe(&mut self) -> Result<Vec<Observation>>;
}

pub struct MultiModalObserver {
    pub step_counter: u64,
}

impl MultiModalObserver {
    pub fn new() -> Self {
        Self { step_counter: 0 }
    }
}

impl Default for MultiModalObserver {
    fn default() -> Self {
        Self::new()
    }
}

impl Observer for MultiModalObserver {
    fn observe(&mut self) -> Result<Vec<Observation>> {
        self.step_counter += 1;
        Ok(vec![Observation {
            timestamp: Utc::now(),
            modality: Modality::Structured,
            content: serde_json::json!({
                "step": self.step_counter,
                "signal": "world_tick",
                "value": (self.step_counter as f64 * 0.1).sin()
            }),
            confidence: 0.95,
            source: "sim_sensor".into(),
        }])
    }
}

/// Reads a single JSON (or plain-text) file on each observe call.
///
/// Useful as a first real adapter: point it at a status file that your system
/// updates, and the kernel will treat its content as the current observation.
pub struct FileObserver {
    path: PathBuf,
    step_counter: u64,
}

impl FileObserver {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            step_counter: 0,
        }
    }
}

impl Observer for FileObserver {
    fn observe(&mut self) -> Result<Vec<Observation>> {
        self.step_counter += 1;
        let raw = std::fs::read_to_string(&self.path).map_err(|e| {
            OmegaError::ExecutionBlocked(format!(
                "FileObserver failed to read {}: {e}",
                self.path.display()
            ))
        })?;

        let content = serde_json::from_str::<serde_json::Value>(&raw).unwrap_or_else(|_| {
            serde_json::json!({
                "raw": raw,
                "step": self.step_counter
            })
        });

        Ok(vec![Observation {
            timestamp: Utc::now(),
            modality: Modality::Structured,
            content,
            confidence: 0.9,
            source: format!("file:{}", self.path.display()),
        }])
    }
}
