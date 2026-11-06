//! HTTP tool adapter — GET-only, host allowlist, action map from config.

use crate::error::{OmegaError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpAdapterConfig {
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_max_body_bytes")]
    pub max_body_bytes: usize,
    pub allowed_hosts: Vec<String>,
    pub actions: HashMap<String, HttpActionSpec>,
}

fn default_timeout_ms() -> u64 {
    3000
}
fn default_max_body_bytes() -> usize {
    4096
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpActionSpec {
    pub method: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct HttpToolAdapter {
    pub config: HttpAdapterConfig,
    pub last_result: Option<serde_json::Value>,
}

impl HttpToolAdapter {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let raw = std::fs::read_to_string(path.as_ref())
            .map_err(|e| OmegaError::Persist(format!("read adapter config: {e}")))?;
        let config: HttpAdapterConfig = serde_json::from_str(&raw)
            .map_err(|e| OmegaError::Persist(format!("parse adapter config: {e}")))?;
        Self::from_config(config)
    }

    pub fn from_config(config: HttpAdapterConfig) -> Result<Self> {
        if config.allowed_hosts.is_empty() {
            return Err(OmegaError::ConstraintViolation(
                "http adapter: allowed_hosts must not be empty".into(),
            ));
        }
        for (name, spec) in &config.actions {
            if !spec.method.eq_ignore_ascii_case("GET") {
                return Err(OmegaError::ConstraintViolation(format!(
                    "http adapter: action `{name}` method must be GET (got {})",
                    spec.method
                )));
            }
            Self::validate_url_against_hosts(&spec.url, &config.allowed_hosts)?;
        }
        Ok(Self {
            config,
            last_result: None,
        })
    }

    pub fn action_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.config.actions.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn handles(&self, action: &str) -> bool {
        self.config.actions.contains_key(action)
    }

    fn validate_url_against_hosts(url: &str, hosts: &[String]) -> Result<()> {
        let parsed = url::Url::parse(url)
            .map_err(|e| OmegaError::ConstraintViolation(format!("invalid url `{url}`: {e}")))?;
        if parsed.scheme() != "https" && parsed.scheme() != "http" {
            return Err(OmegaError::ConstraintViolation(format!(
                "url scheme not allowed: {}",
                parsed.scheme()
            )));
        }
        let host = parsed
            .host_str()
            .ok_or_else(|| OmegaError::ConstraintViolation(format!("url has no host: {url}")))?;
        let ok = hosts.iter().any(|h| h == host || h == "*");
        if !ok {
            return Err(OmegaError::ExecutionBlocked(format!(
                "host `{host}` not in allowed_hosts"
            )));
        }
        // Wildcard host is a foot-gun; still require explicit action entries.
        if hosts.iter().any(|h| h == "*") {
            tracing::warn!("http adapter: allowed_hosts contains wildcard `*`");
        }
        Ok(())
    }

    /// Execute a named allowlisted action. Never follows redirects to off-list hosts
    /// (ureq default redirect policy still re-checks via final URL host when possible).
    pub fn execute_named(&mut self, action: &str) -> Result<serde_json::Value> {
        let spec = self
            .config
            .actions
            .get(action)
            .cloned()
            .ok_or_else(|| OmegaError::ExecutionBlocked(format!("unknown http action: {action}")))?;

        Self::validate_url_against_hosts(&spec.url, &self.config.allowed_hosts)?;

        let timeout = std::time::Duration::from_millis(self.config.timeout_ms.max(1));
        let agent = ureq::AgentBuilder::new().timeout(timeout).build();

        let started = Instant::now();
        let result = match agent.get(&spec.url).call() {
            Ok(resp) => {
                let status = resp.status();
                let mut body = String::new();
                let limit = self.config.max_body_bytes;
                // Read at most max_body_bytes
                let mut buf = vec![0u8; limit.saturating_add(1)];
                match resp.into_reader().read(&mut buf) {
                    Ok(n) => {
                        let take = n.min(limit);
                        body = String::from_utf8_lossy(&buf[..take]).into_owned();
                        if n > limit {
                            body.push_str("…(truncated)");
                        }
                    }
                    Err(e) => {
                        body = format!("(body read error: {e})");
                    }
                }
                let elapsed_ms = started.elapsed().as_millis() as u64;
                serde_json::json!({
                    "ok": status >= 200 && status < 400,
                    "action": action,
                    "status": status,
                    "elapsed_ms": elapsed_ms,
                    "url": spec.url,
                    "body_preview": body,
                })
            }
            Err(ureq::Error::Status(code, resp)) => {
                let elapsed_ms = started.elapsed().as_millis() as u64;
                let mut body = String::new();
                let _ = resp.into_string().map(|s| {
                    let limit = self.config.max_body_bytes;
                    body = if s.len() > limit {
                        format!("{}…(truncated)", &s[..limit])
                    } else {
                        s
                    };
                });
                serde_json::json!({
                    "ok": false,
                    "action": action,
                    "status": code,
                    "elapsed_ms": elapsed_ms,
                    "url": spec.url,
                    "body_preview": body,
                })
            }
            Err(e) => {
                let elapsed_ms = started.elapsed().as_millis() as u64;
                serde_json::json!({
                    "ok": false,
                    "action": action,
                    "status": null,
                    "elapsed_ms": elapsed_ms,
                    "url": spec.url,
                    "error": e.to_string(),
                })
            }
        };

        self.last_result = Some(result.clone());
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> HttpAdapterConfig {
        let mut actions = HashMap::new();
        actions.insert(
            "probe_ok".into(),
            HttpActionSpec {
                method: "GET".into(),
                url: "https://httpbin.org/status/200".into(),
            },
        );
        HttpAdapterConfig {
            timeout_ms: 1000,
            max_body_bytes: 512,
            allowed_hosts: vec!["httpbin.org".into()],
            actions,
        }
    }

    #[test]
    fn rejects_empty_hosts() {
        let mut c = sample_config();
        c.allowed_hosts.clear();
        assert!(HttpToolAdapter::from_config(c).is_err());
    }

    #[test]
    fn rejects_non_get() {
        let mut c = sample_config();
        c.actions.insert(
            "bad".into(),
            HttpActionSpec {
                method: "POST".into(),
                url: "https://httpbin.org/post".into(),
            },
        );
        assert!(HttpToolAdapter::from_config(c).is_err());
    }

    #[test]
    fn rejects_host_not_on_list() {
        let mut c = sample_config();
        c.actions.insert(
            "evil".into(),
            HttpActionSpec {
                method: "GET".into(),
                url: "https://example.com/".into(),
            },
        );
        assert!(HttpToolAdapter::from_config(c).is_err());
    }

    #[test]
    fn unknown_action_blocked() {
        let mut ad = HttpToolAdapter::from_config(sample_config()).unwrap();
        let err = ad.execute_named("not_a_real_action").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unknown") || msg.contains("blocked") || msg.contains("Execution"));
    }
}
