use crate::error::{OmegaError, Result};
use crate::types::{Modality, Observation};
use chrono::Utc;
use std::path::{Path, PathBuf};
use std::time::Duration;

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

/// Snapshot of a directory: file names, sizes, modified times.
///
/// Offline-friendly monitoring adapter — good for log folders or drop directories.
pub struct DirSnapshotObserver {
    path: PathBuf,
    step_counter: u64,
    max_entries: usize,
}

impl DirSnapshotObserver {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            step_counter: 0,
            max_entries: 64,
        }
    }

    pub fn max_entries(mut self, n: usize) -> Self {
        self.max_entries = n.max(1);
        self
    }
}

impl Observer for DirSnapshotObserver {
    fn observe(&mut self) -> Result<Vec<Observation>> {
        self.step_counter += 1;
        let rd = std::fs::read_dir(&self.path).map_err(|e| {
            OmegaError::ExecutionBlocked(format!(
                "DirSnapshotObserver {}: {e}",
                self.path.display()
            ))
        })?;

        let mut entries = Vec::new();
        for ent in rd.flatten().take(self.max_entries) {
            let meta = ent.metadata().ok();
            let modified = meta.as_ref().and_then(|m| m.modified().ok()).and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .map(|d| d.as_secs())
            });
            entries.push(serde_json::json!({
                "name": ent.file_name().to_string_lossy(),
                "is_dir": meta.as_ref().map(|m| m.is_dir()).unwrap_or(false),
                "len": meta.as_ref().map(|m| m.len()).unwrap_or(0),
                "modified_unix": modified,
            }));
        }

        Ok(vec![Observation {
            timestamp: Utc::now(),
            modality: Modality::Structured,
            content: serde_json::json!({
                "step": self.step_counter,
                "dir": self.path.display().to_string(),
                "entry_count": entries.len(),
                "entries": entries,
            }),
            confidence: 0.92,
            source: format!("dir:{}", self.path.display()),
        }])
    }
}

/// GET a URL and treat the body as observation (JSON if possible, else raw text).
///
/// Requires network. Timeout defaults to 5 seconds.
pub struct HttpObserver {
    url: String,
    step_counter: u64,
    timeout: Duration,
}

impl HttpObserver {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            step_counter: 0,
            timeout: Duration::from_secs(5),
        }
    }

    pub fn timeout(mut self, d: Duration) -> Self {
        self.timeout = d;
        self
    }
}

impl Observer for HttpObserver {
    fn observe(&mut self) -> Result<Vec<Observation>> {
        self.step_counter += 1;
        let agent = ureq::AgentBuilder::new().timeout(self.timeout).build();
        let resp = agent.get(&self.url).call().map_err(|e| {
            OmegaError::ExecutionBlocked(format!("HttpObserver GET {}: {e}", self.url))
        })?;
        let status = resp.status();
        let body = resp
            .into_string()
            .map_err(|e| OmegaError::ExecutionBlocked(format!("HttpObserver body: {e}")))?;

        let content = serde_json::from_str::<serde_json::Value>(&body).unwrap_or_else(|_| {
            serde_json::json!({
                "raw": body,
                "http_status": status,
                "step": self.step_counter
            })
        });

        let confidence = if (200..300).contains(&status) {
            0.9
        } else {
            0.5
        };

        Ok(vec![Observation {
            timestamp: Utc::now(),
            modality: Modality::Structured,
            content,
            confidence,
            source: format!("http:{}", self.url),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn dir_snapshot_sees_file() {
        let dir = temp_dir().join(format!("omega-dir-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), b"hi").unwrap();
        let mut obs = DirSnapshotObserver::new(&dir);
        let v = obs.observe().unwrap();
        assert_eq!(v.len(), 1);
        let count = v[0].content["entry_count"].as_u64().unwrap();
        assert!(count >= 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
