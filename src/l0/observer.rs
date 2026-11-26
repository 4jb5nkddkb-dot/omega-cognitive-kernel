#[allow(unused_imports)]
use crate::prelude::*;

#[cfg(feature = "std")]
use crate::error::OmegaError;
use crate::error::Result;
use crate::types::{Modality, Observation};
#[cfg(all(feature = "std", feature = "async"))]
use std::future::Future;
#[cfg(all(
    feature = "std",
    any(feature = "ureq", all(feature = "async", feature = "reqwest"))
))]
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
#[cfg(feature = "std")]
use std::path::{Path, PathBuf};
#[cfg(feature = "std")]
use std::time::Duration;
#[cfg(all(
    feature = "std",
    any(feature = "ureq", all(feature = "async", feature = "reqwest"))
))]
use std::time::Instant;
#[cfg(all(
    feature = "std",
    any(feature = "ureq", all(feature = "async", feature = "reqwest"))
))]
use url::Url;

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

#[cfg(all(
    feature = "std",
    any(feature = "ureq", all(feature = "async", feature = "reqwest"))
))]
const MAX_HTTP_REDIRECTS: usize = 5;

#[cfg(all(
    feature = "std",
    any(feature = "ureq", all(feature = "async", feature = "reqwest"))
))]
fn parse_http_url(raw: &str) -> Result<Url> {
    let url = Url::parse(raw)
        .map_err(|_| OmegaError::ExecutionBlocked("HttpObserver URL is invalid".into()))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(OmegaError::ExecutionBlocked(
            "HttpObserver only accepts credential-free HTTP(S) URLs".into(),
        ));
    }
    Ok(url)
}

#[cfg(all(
    feature = "std",
    any(feature = "ureq", all(feature = "async", feature = "reqwest"))
))]
fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(address) => {
            let octets = address.octets();
            let (a, b, c) = (octets[0], octets[1], octets[2]);
            !(address.is_unspecified()
                || address.is_private()
                || address.is_loopback()
                || address.is_link_local()
                || address.is_broadcast()
                || address.is_multicast()
                || a == 0
                || a >= 224
                || (a == 100 && (64..=127).contains(&b))
                || (a == 192 && (b == 0 || (b == 88 && c == 99)))
                || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
                || (a == 203 && b == 0 && c == 113))
        }
        IpAddr::V6(address) => {
            let segments = address.segments();
            let global_unicast = segments[0] & 0xe000 == 0x2000;
            global_unicast
                && !address.is_unspecified()
                && !address.is_loopback()
                && !address.is_multicast()
                // Exclude 6to4/Teredo and special-purpose/documentation space.
                && segments[0] != 0x2002
                && !(segments[0] == 0x2001
                    && (segments[1] <= 0x01ff || segments[1] == 0x0db8))
        }
    }
}

#[cfg(all(
    feature = "std",
    any(feature = "ureq", all(feature = "async", feature = "reqwest"))
))]
static ACTIVE_HTTP_DNS_LOOKUPS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
#[cfg(all(
    feature = "std",
    any(feature = "ureq", all(feature = "async", feature = "reqwest"))
))]
const MAX_ACTIVE_HTTP_DNS_LOOKUPS: usize = 8;

#[cfg(all(
    feature = "std",
    any(feature = "ureq", all(feature = "async", feature = "reqwest"))
))]
struct HttpDnsLookupSlot;

#[cfg(all(
    feature = "std",
    any(feature = "ureq", all(feature = "async", feature = "reqwest"))
))]
impl Drop for HttpDnsLookupSlot {
    fn drop(&mut self) {
        use std::sync::atomic::Ordering;
        ACTIVE_HTTP_DNS_LOOKUPS.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Run the platform resolver off-thread, enforce its share of the request
/// deadline, and cap abandoned lookups if the OS resolver stalls.
#[cfg(all(
    feature = "std",
    any(feature = "ureq", all(feature = "async", feature = "reqwest"))
))]
fn bounded_http_dns_lookup<F>(timeout: Duration, lookup: F) -> Result<Vec<SocketAddr>>
where
    F: FnOnce() -> std::io::Result<Vec<SocketAddr>> + Send + 'static,
{
    use std::sync::atomic::Ordering;
    use std::sync::mpsc;

    if timeout.is_zero() {
        return Err(OmegaError::ExecutionBlocked(
            "HttpObserver total timeout during DNS lookup".into(),
        ));
    }
    ACTIVE_HTTP_DNS_LOOKUPS
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
            (active < MAX_ACTIVE_HTTP_DNS_LOOKUPS).then_some(active + 1)
        })
        .map_err(|_| {
            OmegaError::ExecutionBlocked("HttpObserver DNS lookup capacity exhausted".into())
        })?;

    let slot = HttpDnsLookupSlot;
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("http-observer-dns".into())
        .spawn(move || {
            let _slot = slot;
            let _ = sender.send(lookup());
        })
        .map_err(|_| OmegaError::ExecutionBlocked("HttpObserver DNS worker unavailable".into()))?;

    match receiver.recv_timeout(timeout) {
        Ok(Ok(addresses)) => Ok(addresses),
        Ok(Err(_)) => Err(OmegaError::ExecutionBlocked(
            "HttpObserver DNS lookup failed".into(),
        )),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(OmegaError::ExecutionBlocked(
            "HttpObserver total timeout during DNS lookup".into(),
        )),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(OmegaError::ExecutionBlocked(
            "HttpObserver DNS worker failed".into(),
        )),
    }
}

/// Resolve once, reject the whole answer set if any address is non-public, and
/// pass this exact vetted set to the HTTP client's resolver to prevent rebinding.
#[cfg(all(
    feature = "std",
    any(feature = "ureq", all(feature = "async", feature = "reqwest"))
))]
fn resolve_public_http_target(url: &Url, dns_timeout: Duration) -> Result<Vec<SocketAddr>> {
    let host = url
        .host_str()
        .ok_or_else(|| OmegaError::ExecutionBlocked("HttpObserver URL has no host".into()))?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| OmegaError::ExecutionBlocked("HttpObserver URL has no port".into()))?;
    let addresses = if let Ok(ip) = host.parse::<IpAddr>() {
        vec![SocketAddr::new(ip, port)]
    } else {
        let host = host.to_owned();
        bounded_http_dns_lookup(dns_timeout, move || {
            (host.as_str(), port)
                .to_socket_addrs()
                .map(|addresses| addresses.collect::<Vec<_>>())
        })?
    };

    if addresses.is_empty() {
        return Err(OmegaError::ExecutionBlocked(
            "HttpObserver DNS lookup returned no addresses".into(),
        ));
    }
    if addresses.iter().any(|address| !is_public_ip(address.ip())) {
        return Err(OmegaError::ExecutionBlocked(
            "HttpObserver target resolves to a non-public address".into(),
        ));
    }

    let mut addresses = addresses;
    addresses.sort_unstable();
    addresses.dedup();
    Ok(addresses)
}

#[cfg(all(
    feature = "std",
    any(feature = "ureq", all(feature = "async", feature = "reqwest"))
))]
fn redirected_http_url(current: &Url, status: u16, location: Option<&str>) -> Result<Option<Url>> {
    if !(300..400).contains(&status) {
        return Ok(None);
    }
    let Some(location) = location else {
        return Ok(None);
    };
    let joined = current
        .join(location)
        .map_err(|_| OmegaError::ExecutionBlocked("HttpObserver redirect URL is invalid".into()))?;
    let next = parse_http_url(joined.as_str())?;
    if current.scheme() == "https" && next.scheme() == "http" {
        return Err(OmegaError::ExecutionBlocked(
            "HttpObserver refused an HTTPS-to-HTTP redirect".into(),
        ));
    }
    Ok(Some(next))
}

#[cfg(all(feature = "std", feature = "ureq"))]
fn fetch_http_sync_with_resolver<F>(
    mut current: Url,
    timeout: Duration,
    connect_timeout: Duration,
    max_response_bytes: usize,
    mut resolve: F,
) -> Result<(u16, Vec<u8>)>
where
    F: FnMut(&Url, Duration) -> Result<Vec<SocketAddr>>,
{
    use std::io::Read;
    use ureq::OrAnyStatus;

    let started = Instant::now();
    for redirect_count in 0..=MAX_HTTP_REDIRECTS {
        let remaining = timeout
            .checked_sub(started.elapsed())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| OmegaError::ExecutionBlocked("HttpObserver total timeout".into()))?;
        let addresses = resolve(&current, remaining)?;
        if addresses.is_empty() {
            return Err(OmegaError::ExecutionBlocked(
                "HttpObserver has no vetted target addresses".into(),
            ));
        }
        let remaining = timeout
            .checked_sub(started.elapsed())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| OmegaError::ExecutionBlocked("HttpObserver total timeout".into()))?;
        let pinned_addresses = addresses.clone();
        let agent = ureq::AgentBuilder::new()
            .redirects(0)
            .try_proxy_from_env(false)
            .timeout_connect(connect_timeout.min(remaining))
            .timeout(remaining)
            .resolver(move |_netloc: &str| -> std::io::Result<Vec<SocketAddr>> {
                Ok(pinned_addresses.clone())
            })
            .build();
        let response = agent
            .get(current.as_str())
            .call()
            .or_any_status()
            .map_err(|_| OmegaError::ExecutionBlocked("HttpObserver request failed".into()))?;
        let status = response.status();
        let location = response.header("Location").map(str::to_owned);
        if let Some(next) = redirected_http_url(&current, status, location.as_deref())? {
            if redirect_count == MAX_HTTP_REDIRECTS {
                return Err(OmegaError::ExecutionBlocked(
                    "HttpObserver exceeded redirect limit".into(),
                ));
            }
            current = next;
            continue;
        }

        let read_limit = u64::try_from(max_response_bytes.saturating_add(1)).unwrap_or(u64::MAX);
        let mut body = Vec::with_capacity(max_response_bytes.min(8192));
        response
            .into_reader()
            .take(read_limit)
            .read_to_end(&mut body)
            .map_err(|_| OmegaError::ExecutionBlocked("HttpObserver body read failed".into()))?;
        if body.len() > max_response_bytes {
            return Err(OmegaError::ExecutionBlocked(
                "HttpObserver response body exceeds configured limit".into(),
            ));
        }
        return Ok((status, body));
    }
    Err(OmegaError::ExecutionBlocked(
        "HttpObserver exceeded redirect limit".into(),
    ))
}

#[cfg(all(feature = "std", feature = "ureq"))]
fn fetch_http_sync(
    url: Url,
    timeout: Duration,
    connect_timeout: Duration,
    max_response_bytes: usize,
) -> Result<(u16, Vec<u8>)> {
    fetch_http_sync_with_resolver(
        url,
        timeout,
        connect_timeout,
        max_response_bytes,
        resolve_public_http_target,
    )
}

/// GET a URL and treat the body as observation (JSON if possible, else raw text).
///
/// Requires network. Only public HTTP(S) targets are allowed. Requests have a
/// 5-second total timeout, a 2-second connect timeout, and a 1 MiB body limit
/// by default. Redirects are revalidated individually.
#[cfg(feature = "std")]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
#[allow(dead_code)]
pub struct HttpObserver {
    url: String,
    step_counter: u64,
    timeout: Duration,
    connect_timeout: Duration,
    max_response_bytes: usize,
    #[cfg(feature = "ureq")]
    _ureq: (),
}

#[cfg(feature = "std")]
impl HttpObserver {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            step_counter: 0,
            timeout: Duration::from_secs(5),
            connect_timeout: Duration::from_secs(2),
            max_response_bytes: 1024 * 1024,
            #[cfg(feature = "ureq")]
            _ureq: (),
        }
    }

    pub fn timeout(mut self, d: Duration) -> Self {
        self.timeout = d;
        self
    }

    pub fn connect_timeout(mut self, d: Duration) -> Self {
        self.connect_timeout = d;
        self
    }

    pub fn max_response_bytes(mut self, max: usize) -> Self {
        self.max_response_bytes = max.max(1);
        self
    }
}

#[cfg(all(feature = "std", feature = "ureq"))]
impl Observer for HttpObserver {
    fn observe(&mut self) -> Result<Vec<Observation>> {
        self.step_counter += 1;
        let url = parse_http_url(&self.url)?;
        let (status, body) = fetch_http_sync(
            url,
            self.timeout,
            self.connect_timeout,
            self.max_response_bytes,
        )?;
        let body = String::from_utf8_lossy(&body).into_owned();

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

#[cfg(all(feature = "std", feature = "async", feature = "reqwest"))]
async fn fetch_http_async_with_resolver<F>(
    mut current: Url,
    timeout: Duration,
    connect_timeout: Duration,
    max_response_bytes: usize,
    mut resolve: F,
) -> Result<(u16, Vec<u8>)>
where
    F: FnMut(&Url, Duration) -> Result<Vec<SocketAddr>> + Send,
{
    use futures::StreamExt;

    let started = Instant::now();
    for redirect_count in 0..=MAX_HTTP_REDIRECTS {
        let remaining = timeout
            .checked_sub(started.elapsed())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| {
                OmegaError::ExecutionBlocked("AsyncHttpObserver total timeout".into())
            })?;
        let addresses = resolve(&current, remaining)?;
        if addresses.is_empty() {
            return Err(OmegaError::ExecutionBlocked(
                "AsyncHttpObserver has no vetted target addresses".into(),
            ));
        }
        let remaining = timeout
            .checked_sub(started.elapsed())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| {
                OmegaError::ExecutionBlocked("AsyncHttpObserver total timeout".into())
            })?;
        let mut builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .connect_timeout(connect_timeout.min(remaining))
            .timeout(remaining);
        if let Some(host) = current.host_str() {
            if host.parse::<IpAddr>().is_err() {
                builder = builder.resolve_to_addrs(host, &addresses);
            }
        }
        let client = builder.build().map_err(|_| {
            OmegaError::ExecutionBlocked("AsyncHttpObserver client setup failed".into())
        })?;
        let response =
            client.get(current.clone()).send().await.map_err(|_| {
                OmegaError::ExecutionBlocked("AsyncHttpObserver request failed".into())
            })?;
        let status = response.status().as_u16();
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .map(|value| {
                value.to_str().map(str::to_owned).map_err(|_| {
                    OmegaError::ExecutionBlocked(
                        "AsyncHttpObserver redirect location is invalid".into(),
                    )
                })
            })
            .transpose()?;
        if let Some(next) = redirected_http_url(&current, status, location.as_deref())? {
            if redirect_count == MAX_HTTP_REDIRECTS {
                return Err(OmegaError::ExecutionBlocked(
                    "AsyncHttpObserver exceeded redirect limit".into(),
                ));
            }
            current = next;
            continue;
        }

        if response
            .content_length()
            .is_some_and(|length| length > max_response_bytes as u64)
        {
            return Err(OmegaError::ExecutionBlocked(
                "AsyncHttpObserver response body exceeds configured limit".into(),
            ));
        }
        let mut stream = response.bytes_stream();
        let mut body = Vec::with_capacity(max_response_bytes.min(8192));
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| {
                OmegaError::ExecutionBlocked("AsyncHttpObserver body read failed".into())
            })?;
            if body.len().saturating_add(chunk.len()) > max_response_bytes {
                return Err(OmegaError::ExecutionBlocked(
                    "AsyncHttpObserver response body exceeds configured limit".into(),
                ));
            }
            body.extend_from_slice(&chunk);
        }
        return Ok((status, body));
    }
    Err(OmegaError::ExecutionBlocked(
        "AsyncHttpObserver exceeded redirect limit".into(),
    ))
}

#[cfg(all(feature = "std", feature = "async", feature = "reqwest"))]
async fn fetch_http_async(
    url: Url,
    timeout: Duration,
    connect_timeout: Duration,
    max_response_bytes: usize,
) -> Result<(u16, Vec<u8>)> {
    fetch_http_async_with_resolver(
        url,
        timeout,
        connect_timeout,
        max_response_bytes,
        resolve_public_http_target,
    )
    .await
}

/// Async HTTP observer using reqwest with the same address and resource limits
/// as [`HttpObserver`].
#[cfg(all(feature = "std", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "std", feature = "async"))))]
pub struct AsyncHttpObserver {
    url: String,
    step_counter: u64,
    timeout: Duration,
    connect_timeout: Duration,
    max_response_bytes: usize,
}

#[cfg(all(feature = "std", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "std", feature = "async"))))]
impl AsyncHttpObserver {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            step_counter: 0,
            timeout: Duration::from_secs(5),
            connect_timeout: Duration::from_secs(2),
            max_response_bytes: 1024 * 1024,
        }
    }

    pub fn timeout(mut self, d: Duration) -> Self {
        self.timeout = d;
        self
    }

    pub fn connect_timeout(mut self, d: Duration) -> Self {
        self.connect_timeout = d;
        self
    }

    pub fn max_response_bytes(mut self, max: usize) -> Self {
        self.max_response_bytes = max.max(1);
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
        let connect_timeout = self.connect_timeout;
        let max_response_bytes = self.max_response_bytes;
        async move {
            let parsed_url = parse_http_url(&url)?;
            let (status, body) =
                fetch_http_async(parsed_url, timeout, connect_timeout, max_response_bytes).await?;
            let body = String::from_utf8_lossy(&body).into_owned();
            let content = serde_json::from_str::<serde_json::Value>(&body).unwrap_or_else(|_| {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[cfg(any(feature = "ureq", all(feature = "async", feature = "reqwest")))]
    fn test_response(
        status: &str,
        headers: &str,
        body: &str,
        delay: Duration,
    ) -> (String, Duration) {
        (
            format!(
                "HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            ),
            delay,
        )
    }

    #[cfg(any(feature = "ureq", all(feature = "async", feature = "reqwest")))]
    fn start_test_http_server(
        responses: Vec<(String, Duration)>,
    ) -> (SocketAddr, std::thread::JoinHandle<()>) {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let thread = std::thread::spawn(move || {
            for (response, delay) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0u8; 4096];
                let _ = stream.read(&mut request);
                let (headers, body) = response.split_once("\r\n\r\n").unwrap();
                stream.write_all(headers.as_bytes()).unwrap();
                stream.write_all(b"\r\n\r\n").unwrap();
                stream.flush().unwrap();
                if !delay.is_zero() {
                    std::thread::sleep(delay);
                }
                stream.write_all(body.as_bytes()).unwrap();
                stream.flush().unwrap();
            }
        });
        (address, thread)
    }

    #[cfg(any(feature = "ureq", all(feature = "async", feature = "reqwest")))]
    fn chunked_test_response(body: &str) -> (String, Duration) {
        (
            format!(
                "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n{}\r\n0\r\n\r\n",
                body.len(), body
            ),
            Duration::ZERO,
        )
    }

    #[cfg(feature = "ureq")]
    #[test]
    fn http_target_policy_rejects_private_reserved_and_credential_urls() {
        for address in [
            "0.0.0.0",
            "10.0.0.1",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.169.254",
            "172.16.0.1",
            "192.168.1.1",
            "198.18.0.1",
            "224.0.0.1",
            "240.0.0.1",
            "::1",
            "fd00::1",
            "fe80::1",
            "2001:db8::1",
            "2002::1",
        ] {
            assert!(!is_public_ip(address.parse().unwrap()), "{address}");
        }
        for address in ["1.1.1.1", "8.8.8.8", "2606:4700:4700::1111"] {
            assert!(is_public_ip(address.parse().unwrap()), "{address}");
        }
        for raw in [
            "http://127.0.0.1/",
            "http://169.254.169.254/latest/meta-data/",
            "http://[::1]/",
        ] {
            let url = parse_http_url(raw).unwrap();
            assert!(
                resolve_public_http_target(&url, Duration::from_secs(1)).is_err(),
                "{raw}"
            );
        }
        assert!(parse_http_url("file:///etc/passwd").is_err());
        assert!(parse_http_url("http://user:password@example.com/").is_err());
    }

    #[cfg(any(feature = "ureq", all(feature = "async", feature = "reqwest")))]
    #[test]
    fn http_dns_lookup_obeys_deadline_and_releases_bounded_worker() {
        use std::sync::atomic::Ordering;

        let baseline = ACTIVE_HTTP_DNS_LOOKUPS.load(Ordering::Acquire);
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let result = bounded_http_dns_lookup(Duration::from_millis(10), move || {
            let _ = started_tx.send(());
            std::thread::sleep(Duration::from_millis(50));
            Ok(Vec::new())
        });
        assert!(result.is_err());
        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("DNS worker should start");

        let deadline = Instant::now() + Duration::from_secs(1);
        while ACTIVE_HTTP_DNS_LOOKUPS.load(Ordering::Acquire) > baseline
            && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(ACTIVE_HTTP_DNS_LOOKUPS.load(Ordering::Acquire), baseline);
    }

    #[cfg(any(feature = "ureq", all(feature = "async", feature = "reqwest")))]
    #[test]
    fn http_redirect_refuses_https_downgrade() {
        let current = Url::parse("https://example.com/start").unwrap();
        assert!(redirected_http_url(&current, 307, Some("http://example.com/final")).is_err());
    }

    #[cfg(feature = "ureq")]
    #[test]
    fn http_redirect_is_revalidated_before_following() {
        let (address, server) = start_test_http_server(vec![test_response(
            "302 Found",
            "Location: http://127.0.0.1:1/private\r\n",
            "",
            Duration::ZERO,
        )]);
        let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = calls.clone();
        let result = fetch_http_sync_with_resolver(
            Url::parse("http://safe.example/start").unwrap(),
            Duration::from_secs(2),
            Duration::from_millis(300),
            1024,
            move |url, remaining| {
                seen.lock().unwrap().push(url.to_string());
                if url.host_str() == Some("safe.example") {
                    Ok(vec![address])
                } else {
                    resolve_public_http_target(url, remaining)
                }
            },
        );
        assert!(result.is_err());
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 2, "each redirect target must be revalidated");
        assert!(calls[1].contains("127.0.0.1"));
        drop(calls);
        server.join().unwrap();
    }

    #[cfg(feature = "ureq")]
    #[test]
    fn http_redirects_are_followed_only_after_each_safe_validation() {
        let (address, server) = start_test_http_server(vec![
            test_response("302 Found", "Location: /final\r\n", "", Duration::ZERO),
            test_response("200 OK", "", "ok", Duration::ZERO),
        ]);
        let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = calls.clone();
        let (status, body) = fetch_http_sync_with_resolver(
            Url::parse("http://safe.example/start").unwrap(),
            Duration::from_secs(2),
            Duration::from_millis(300),
            16,
            move |url, _remaining| {
                seen.lock().unwrap().push(url.path().to_owned());
                Ok(vec![address])
            },
        )
        .unwrap();
        assert_eq!(status, 200);
        assert_eq!(body, b"ok");
        assert_eq!(*calls.lock().unwrap(), vec!["/start", "/final"]);
        server.join().unwrap();
    }

    #[cfg(feature = "ureq")]
    #[test]
    fn http_response_body_limit_and_total_timeout_are_enforced() {
        let (address, server) = start_test_http_server(vec![chunked_test_response("abcdef")]);
        let result = fetch_http_sync_with_resolver(
            Url::parse("http://safe.example/large").unwrap(),
            Duration::from_secs(2),
            Duration::from_millis(300),
            5,
            move |_, _remaining| Ok(vec![address]),
        );
        assert!(result.is_err());
        server.join().unwrap();

        let (address, server) = start_test_http_server(vec![test_response(
            "200 OK",
            "",
            "slow",
            Duration::from_millis(300),
        )]);
        let result = fetch_http_sync_with_resolver(
            Url::parse("http://safe.example/slow").unwrap(),
            Duration::from_millis(100),
            Duration::from_millis(100),
            16,
            move |_, _remaining| Ok(vec![address]),
        );
        assert!(result.is_err());
        server.join().unwrap();
    }

    #[cfg(all(feature = "std", feature = "async", feature = "reqwest"))]
    #[tokio::test]
    async fn async_http_redirect_and_streaming_body_limit_are_enforced() {
        let (address, server) = start_test_http_server(vec![test_response(
            "302 Found",
            "Location: http://127.0.0.1:1/private\r\n",
            "",
            Duration::ZERO,
        )]);
        let result = fetch_http_async_with_resolver(
            Url::parse("http://safe.example/start").unwrap(),
            Duration::from_secs(2),
            Duration::from_millis(300),
            1024,
            move |url, remaining| {
                if url.host_str() == Some("safe.example") {
                    Ok(vec![address])
                } else {
                    resolve_public_http_target(url, remaining)
                }
            },
        )
        .await;
        assert!(result.is_err());
        server.join().unwrap();

        let (address, server) = start_test_http_server(vec![chunked_test_response("abcdef")]);
        let result = fetch_http_async_with_resolver(
            Url::parse("http://safe.example/large").unwrap(),
            Duration::from_secs(2),
            Duration::from_millis(300),
            5,
            move |_, _remaining| Ok(vec![address]),
        )
        .await;
        assert!(result.is_err());
        server.join().unwrap();

        let (address, server) = start_test_http_server(vec![test_response(
            "200 OK",
            "",
            "slow",
            Duration::from_millis(200),
        )]);
        let result = fetch_http_async_with_resolver(
            Url::parse("http://safe.example/slow").unwrap(),
            Duration::from_millis(50),
            Duration::from_millis(50),
            16,
            move |_, _remaining| Ok(vec![address]),
        )
        .await;
        assert!(result.is_err());
        server.join().unwrap();
    }

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
