#[allow(unused_imports)]
use crate::prelude::*;

#[cfg(feature = "std")]
use crate::error::OmegaError;
use crate::error::Result;
use crate::types::{Modality, Observation};
#[cfg(all(feature = "std", feature = "async"))]
use std::future::Future;
#[cfg(feature = "std")]
use std::path::{Path, PathBuf};
#[cfg(feature = "std")]
use std::time::Duration;

/// External world observation interface (synchronous).
///
/// Implement this trait to feed real sensors, files, HTTP status, or any other
/// source into the kernel. The default `MultiModalObserver` is a pure simulation.
pub trait Observer {
    fn observe(&mut self) -> Result<Vec<Observation>>;
}

/// External world observation interface (asynchronous).
///
/// Async version of [`Observer`] for use with async runtimes (tokio, async-std).
/// Requires the `async` feature (which implies `std`).
#[cfg(all(feature = "std", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "std", feature = "async"))))]
pub trait AsyncObserver {
    fn observe_async(&mut self) -> impl Future<Output = Result<Vec<Observation>>> + Send;
}

/// Adapter to use a synchronous [`Observer`] as an [`AsyncObserver`] by running
/// the blocking call on a thread pool (tokio).
#[cfg(all(feature = "std", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
pub struct AsyncObserverAdapter<O> {
    inner: Option<O>,
}

#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
impl<O> AsyncObserverAdapter<O> {
    pub fn new(observer: O) -> Self {
        Self {
            inner: Some(observer),
        }
    }

    pub fn into_inner(mut self) -> O {
        self.inner
            .take()
            .expect("AsyncObserverAdapter already consumed")
    }
}

#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
impl<O: Observer + Send + 'static> AsyncObserver for AsyncObserverAdapter<O> {
    fn observe_async(&mut self) -> impl Future<Output = Result<Vec<Observation>>> + Send {
        let mut observer = self
            .inner
            .take()
            .expect("AsyncObserverAdapter already consumed");
        async move {
            let result = tokio::task::spawn_blocking(move || observer.observe()).await;
            match result {
                Ok(r) => r,
                Err(e) => Err(crate::error::OmegaError::Other(anyhow::anyhow!(e))),
            }
        }
    }
}

/// Blanket implementation: any sync `Observer` can be used as an `AsyncObserver`
/// by wrapping it in `AsyncObserverAdapter`.
#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
impl<O: Observer + Send + 'static> From<O> for AsyncObserverAdapter<O> {
    fn from(observer: O) -> Self {
        Self::new(observer)
    }
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
            timestamp: crate::now_utc(),
            modality: Modality::Structured,
            content: serde_json::json!({
                "step": self.step_counter,
                "signal": "world_tick",
                "value": libm::sin(self.step_counter as f64 * 0.1)
            }),
            confidence: 0.95,
            source: "sim_sensor".into(),
        }])
    }
}

/// Reads a single JSON (or plain-text) file on each observe call.
#[cfg(feature = "std")]
pub struct FileObserver {
    path: PathBuf,
    step_counter: u64,
}

#[cfg(feature = "std")]
impl FileObserver {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            step_counter: 0,
        }
    }
}

#[cfg(feature = "std")]
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
            timestamp: crate::now_utc(),
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
#[cfg(feature = "std")]
pub struct DirSnapshotObserver {
    path: PathBuf,
    step_counter: u64,
    max_entries: usize,
}

#[cfg(feature = "std")]
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

#[cfg(feature = "std")]
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
            timestamp: crate::now_utc(),
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
#[cfg(feature = "std")]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub struct HttpObserver {
    url: String,
    step_counter: u64,
    timeout: Duration,
}

#[cfg(feature = "std")]
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

#[cfg(feature = "std")]
impl Observer for HttpObserver {
    fn observe(&mut self) -> Result<Vec<Observation>> {
        self.step_counter += 1;
        // ureq 2.x: use timeout_connect / timeout_read (not a single `.timeout`).
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(self.timeout)
            .timeout_read(self.timeout)
            .build();
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
            timestamp: crate::now_utc(),
            modality: Modality::Structured,
            content,
            confidence,
            source: format!("http:{}", self.url),
        }])
    }
}

/// Async file observer using tokio's async filesystem.
#[cfg(all(feature = "std", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "std", feature = "async"))))]
pub struct AsyncFileObserver {
    path: PathBuf,
    step_counter: u64,
}

#[cfg(all(feature = "std", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "std", feature = "async"))))]
impl AsyncFileObserver {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            step_counter: 0,
        }
    }
}

#[cfg(all(feature = "std", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "std", feature = "async"))))]
impl AsyncObserver for AsyncFileObserver {
    fn observe_async(&mut self) -> impl Future<Output = Result<Vec<Observation>>> + Send {
        let path = self.path.clone();
        self.step_counter += 1;
        let step = self.step_counter;
        async move {
            let raw = tokio::fs::read_to_string(&path).await.map_err(|e| {
                OmegaError::ExecutionBlocked(format!(
                    "AsyncFileObserver failed to read {}: {e}",
                    path.display()
                ))
            })?;

            let content = serde_json::from_str::<serde_json::Value>(&raw).unwrap_or_else(|_| {
                serde_json::json!({
                    "raw": raw,
                    "step": step
                })
            });

            Ok(vec![Observation {
                timestamp: crate::now_utc(),
                modality: Modality::Structured,
                content,
                confidence: 0.9,
                source: format!("async_file:{}", path.display()),
            }])
        }
    }
}

/// Async directory snapshot observer using tokio's async filesystem.
#[cfg(all(feature = "std", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "std", feature = "async"))))]
pub struct AsyncDirSnapshotObserver {
    path: PathBuf,
    step_counter: u64,
    max_entries: usize,
}

#[cfg(all(feature = "std", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "std", feature = "async"))))]
impl AsyncDirSnapshotObserver {
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

#[cfg(all(feature = "std", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "std", feature = "async"))))]
impl AsyncObserver for AsyncDirSnapshotObserver {
    fn observe_async(&mut self) -> impl Future<Output = Result<Vec<Observation>>> + Send {
        let path = self.path.clone();
        self.step_counter += 1;
        let step = self.step_counter;
        let max_entries = self.max_entries;
        async move {
            let mut rd = tokio::fs::read_dir(&path).await.map_err(|e| {
                OmegaError::ExecutionBlocked(format!(
                    "AsyncDirSnapshotObserver {}: {e}",
                    path.display()
                ))
            })?;

            let mut entries = Vec::new();
            while let Some(ent) = rd.next_entry().await.map_err(|e| {
                OmegaError::ExecutionBlocked(format!("AsyncDirSnapshotObserver read entry: {e}"))
            })? {
                if entries.len() >= max_entries {
                    break;
                }
                let meta = ent.metadata().await.ok();
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
                timestamp: crate::now_utc(),
                modality: Modality::Structured,
                content: serde_json::json!({
                    "step": step,
                    "dir": path.display().to_string(),
                    "entry_count": entries.len(),
                    "entries": entries,
                }),
                confidence: 0.92,
                source: format!("async_dir:{}", path.display()),
            }])
        }
    }
}

/// Async HTTP observer using reqwest (optional, falls back to ureq in thread pool).
#[cfg(all(feature = "std", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "std", feature = "async"))))]
pub struct AsyncHttpObserver {
    url: String,
    step_counter: u64,
    timeout: Duration,
}

#[cfg(all(feature = "std", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "std", feature = "async"))))]
impl AsyncHttpObserver {
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

#[cfg(all(feature = "std", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "std", feature = "async"))))]
impl AsyncObserver for AsyncHttpObserver {
    fn observe_async(&mut self) -> impl Future<Output = Result<Vec<Observation>>> + Send {
        let url = self.url.clone();
        self.step_counter += 1;
        let step = self.step_counter;
        let timeout = self.timeout;
        async move {
            // Use reqwest if available, otherwise fall back to ureq in thread pool
            #[cfg(feature = "reqwest")]
            {
                let client = reqwest::Client::builder()
                    .timeout(timeout)
                    .build()
                    .map_err(|e| {
                        OmegaError::ExecutionBlocked(format!("AsyncHttpObserver client: {e}"))
                    })?;
                let resp = client.get(&url).send().await.map_err(|e| {
                    OmegaError::ExecutionBlocked(format!("AsyncHttpObserver GET {}: {e}", url))
                })?;
                let status = resp.status().as_u16();
                let body = resp.text().await.map_err(|e| {
                    OmegaError::ExecutionBlocked(format!("AsyncHttpObserver body: {e}"))
                })?;
                let content =
                    serde_json::from_str::<serde_json::Value>(&body).unwrap_or_else(|_| {
                        serde_json::json!({
                            "raw": body,
                            "http_status": status,
                            "step": step
                        })
                    });
                let confidence = if (200..300).contains(&status) {
                    0.9
                } else {
                    0.5
                };
                Ok(vec![Observation {
                    timestamp: crate::now_utc(),
                    modality: Modality::Structured,
                    content,
                    confidence,
                    source: format!("async_http:{}", url),
                }])
            }
            #[cfg(not(feature = "reqwest"))]
            {
                // Fallback: run ureq in blocking thread pool
                let url_for_closure = url.clone();
                let url_for_source = url.clone();
                let timeout = timeout;
                let result = tokio::task::spawn_blocking(move || {
                    let agent = ureq::AgentBuilder::new()
                        .timeout_connect(timeout)
                        .timeout_read(timeout)
                        .build();
                    let resp = agent.get(&url_for_closure).call().map_err(|e| {
                        OmegaError::ExecutionBlocked(format!(
                            "AsyncHttpObserver GET {}: {e}",
                            url_for_closure
                        ))
                    })?;
                    let status = resp.status();
                    let body = resp.into_string().map_err(|e| {
                        OmegaError::ExecutionBlocked(format!("AsyncHttpObserver body: {e}"))
                    })?;
                    Ok::<(u16, String), OmegaError>((status, body))
                })
                .await;
                let (status, body) =
                    result.map_err(|e| OmegaError::Other(anyhow::anyhow!(e)))??;
                let content =
                    serde_json::from_str::<serde_json::Value>(&body).unwrap_or_else(|_| {
                        serde_json::json!({
                            "raw": body,
                            "http_status": status,
                            "step": step
                        })
                    });
                let confidence = if (200..300).contains(&status) {
                    0.9
                } else {
                    0.5
                };
                Ok(vec![Observation {
                    timestamp: crate::now_utc(),
                    modality: Modality::Structured,
                    content,
                    confidence,
                    source: format!("async_http:{}", url_for_source),
                }])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn dir_snapshot_sees_file() {
        let dir = temp_dir().join(format!("omega-dir-{}", crate::new_uuid()));
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
