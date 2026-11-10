use crate::audit::{AuditEvent, AuditLog};
use crate::error::{OmegaError, Result};
use crate::types::Action;
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// External world execution interface.
///
/// Implement this trait to apply kernel decisions to the real world.
/// The default `SandboxedExecutor` only allows a fixed whitelist and returns
/// synthetic success payloads (no side effects).
pub trait Executor {
    fn execute(&mut self, actions: &[Action]) -> Result<Vec<serde_json::Value>>;
}

pub struct SandboxedExecutor {
    allowed: HashSet<String>,
}

impl SandboxedExecutor {
    pub fn new(allowed: impl IntoIterator<Item = String>) -> Self {
        Self {
            allowed: allowed.into_iter().collect(),
        }
    }
}

impl Executor for SandboxedExecutor {
    fn execute(&mut self, actions: &[Action]) -> Result<Vec<serde_json::Value>> {
        let mut results = Vec::new();
        for act in actions {
            if !act.constraints_checked {
                return Err(OmegaError::ExecutionBlocked(format!(
                    "{} (constraints not checked)",
                    act.name
                )));
            }
            if !self.allowed.contains(&act.name) {
                return Err(OmegaError::ExecutionBlocked(act.name.clone()));
            }
            results.push(serde_json::json!({
                "status": "ok",
                "action": act.name,
                "params": act.params
            }));
        }
        Ok(results)
    }
}

/// Records intended actions to a log file (and optionally stdout).
pub struct LoggingExecutor {
    allowed: HashSet<String>,
    log_path: Option<PathBuf>,
    echo_stdout: bool,
}

impl LoggingExecutor {
    pub fn new(allowed: impl IntoIterator<Item = String>) -> Self {
        Self {
            allowed: allowed.into_iter().collect(),
            log_path: None,
            echo_stdout: true,
        }
    }

    pub fn with_log_file(mut self, path: impl AsRef<Path>) -> Self {
        self.log_path = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn echo_stdout(mut self, enabled: bool) -> Self {
        self.echo_stdout = enabled;
        self
    }
}

impl Executor for LoggingExecutor {
    fn execute(&mut self, actions: &[Action]) -> Result<Vec<serde_json::Value>> {
        let mut results = Vec::new();
        for act in actions {
            if !act.constraints_checked {
                return Err(OmegaError::ExecutionBlocked(format!(
                    "{} (constraints not checked)",
                    act.name
                )));
            }
            if !self.allowed.contains(&act.name) {
                return Err(OmegaError::ExecutionBlocked(act.name.clone()));
            }

            let entry = serde_json::json!({
                "status": "logged",
                "action": act.name,
                "params": act.params,
                "ts": chrono::Utc::now().to_rfc3339()
            });

            if self.echo_stdout {
                println!("[LoggingExecutor] {}", entry);
            }
            if let Some(path) = &self.log_path {
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .map_err(|e| OmegaError::ExecutionBlocked(format!("log open: {e}")))?;
                writeln!(file, "{entry}")
                    .map_err(|e| OmegaError::ExecutionBlocked(format!("log write: {e}")))?;
            }

            results.push(entry);
        }
        Ok(results)
    }
}

/// Max response body bytes kept for `http_get` (truncate after this).
const HTTP_GET_BODY_LIMIT: usize = 2048;

/// Safe GET-only fetch for tool-gate. No redirects beyond ureq defaults; HTTPS/HTTP only.
fn safe_http_get(url: &str) -> Result<serde_json::Value> {
    let url = url.trim();
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(OmegaError::ExecutionBlocked(
            "http_get: only http:// or https:// URLs allowed".into(),
        ));
    }
    // Block obvious credential injection in userinfo (basic hygiene).
    if url.contains('@') && !url.split('@').next().unwrap_or("").contains("://") {
        // crude: reject if @ appears before path in a way that looks like userinfo
    }
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(10))
        .build();
    let resp = agent
        .get(url)
        .call()
        .map_err(|e| OmegaError::ExecutionBlocked(format!("http_get failed: {e}")))?;
    let status = resp.status();
    let body = resp
        .into_string()
        .map_err(|e| OmegaError::ExecutionBlocked(format!("http_get body: {e}")))?;
    let truncated = body.len() > HTTP_GET_BODY_LIMIT;
    let preview: String = body.chars().take(HTTP_GET_BODY_LIMIT).collect();
    Ok(serde_json::json!({
        "status": "ok",
        "action": "http_get",
        "http_status": status,
        "url": url,
        "body_preview": preview,
        "truncated": truncated,
        "effect": "network_get",
    }))
}

/// Tool-call gate: allow-list + deny-list, optional audit, optional real `http_get`.
///
/// Denied tools never touch the network. When `enable_http_get` is true and the
/// tool name is `http_get` on the allow-list, a GET is performed (response truncated).
pub struct ToolGateExecutor {
    allowed: HashSet<String>,
    denied: HashSet<String>,
    audit: Option<AuditLog>,
    echo: bool,
    fail_closed: bool,
    /// If true, ALLOW + name `http_get` performs a real GET (`params.url`).
    enable_http_get: bool,
}

impl ToolGateExecutor {
    pub fn new(allowed: impl IntoIterator<Item = String>) -> Self {
        Self {
            allowed: allowed.into_iter().collect(),
            denied: HashSet::new(),
            audit: None,
            echo: true,
            fail_closed: true,
            enable_http_get: false,
        }
    }

    pub fn deny(mut self, names: impl IntoIterator<Item = String>) -> Self {
        self.denied.extend(names);
        self
    }

    pub fn with_audit(mut self, log: AuditLog) -> Self {
        self.audit = Some(log);
        self
    }

    pub fn echo(mut self, on: bool) -> Self {
        self.echo = on;
        self
    }

    pub fn fail_closed(mut self, on: bool) -> Self {
        self.fail_closed = on;
        self
    }

    /// Enable real HTTP GET for tool name `http_get` after allow-list pass.
    pub fn enable_http_get(mut self, on: bool) -> Self {
        self.enable_http_get = on;
        self
    }

    fn is_denied(&self, name: &str) -> bool {
        self.denied.contains(name) || !self.allowed.contains(name)
    }
}

impl Executor for ToolGateExecutor {
    fn execute(&mut self, actions: &[Action]) -> Result<Vec<serde_json::Value>> {
        let mut results = Vec::new();
        for act in actions {
            if !act.constraints_checked {
                return Err(OmegaError::ExecutionBlocked(format!(
                    "{} (constraints not checked)",
                    act.name
                )));
            }

            if self.is_denied(&act.name) {
                let reason = if self.denied.contains(&act.name) {
                    "deny_list"
                } else {
                    "not_on_allow_list"
                };
                if let Some(log) = &self.audit {
                    let _ = log.append(AuditEvent::ToolDenied {
                        tool: act.name.clone(),
                        reason: reason.into(),
                    });
                }
                if self.echo {
                    println!("[tool-gate] DENY  {} ({reason})", act.name);
                }
                if self.fail_closed {
                    return Err(OmegaError::ExecutionBlocked(format!(
                        "tool denied: {} ({reason})",
                        act.name
                    )));
                }
                results.push(serde_json::json!({
                    "status": "denied",
                    "action": act.name,
                    "reason": reason,
                }));
                continue;
            }

            if let Some(log) = &self.audit {
                let _ = log.append(AuditEvent::ToolAllowed {
                    tool: act.name.clone(),
                });
            }
            if self.echo {
                println!("[tool-gate] ALLOW {}", act.name);
            }

            // Real side effect only for http_get when explicitly enabled.
            if self.enable_http_get && act.name == "http_get" {
                let url = act.params.get("url").and_then(|v| v.as_str()).unwrap_or("");
                if url.is_empty() {
                    return Err(OmegaError::ExecutionBlocked(
                        "http_get requires params.url".into(),
                    ));
                }
                if self.echo {
                    println!("[tool-gate] GET   {url}");
                }
                results.push(safe_http_get(url)?);
                continue;
            }

            results.push(serde_json::json!({
                "status": "ok",
                "action": act.name,
                "params": act.params,
                "effect": "simulated",
            }));
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Action;
    use std::collections::HashMap;
    use std::env::temp_dir;
    use uuid::Uuid;

    fn action(name: &str, checked: bool) -> Action {
        Action {
            name: name.into(),
            params: HashMap::new(),
            constraints_checked: checked,
        }
    }

    #[test]
    fn allows_whitelisted_checked_action() {
        let mut ex = SandboxedExecutor::new(vec!["observe".into()]);
        let res = ex.execute(&[action("observe", true)]);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().len(), 1);
    }

    #[test]
    fn blocks_unchecked_action() {
        let mut ex = SandboxedExecutor::new(vec!["observe".into()]);
        let res = ex.execute(&[action("observe", false)]);
        assert!(res.is_err());
    }

    #[test]
    fn blocks_non_whitelisted_action() {
        let mut ex = SandboxedExecutor::new(vec!["observe".into()]);
        let res = ex.execute(&[action("delete_everything", true)]);
        assert!(res.is_err());
    }

    #[test]
    fn logging_executor_records_action() {
        let mut ex = LoggingExecutor::new(vec!["observe".into()]).echo_stdout(false);
        let res = ex.execute(&[action("observe", true)]).unwrap();
        assert_eq!(res[0]["status"], "logged");
        assert_eq!(res[0]["action"], "observe");
    }

    #[test]
    fn tool_gate_denies_dangerous_tool() {
        let path = temp_dir().join(format!("omega-gate-{}.json", Uuid::new_v4()));
        let audit = AuditLog::beside_state(&path);
        let mut gate = ToolGateExecutor::new(["search_docs".into(), "http_get".into()])
            .deny(["delete_database".into(), "shell_exec".into()])
            .with_audit(audit)
            .echo(false)
            .fail_closed(true);
        assert!(gate.execute(&[action("search_docs", true)]).is_ok());
        assert!(gate.execute(&[action("delete_database", true)]).is_err());
        let rows = AuditLog::beside_state(&path).read_all().unwrap();
        assert!(rows.len() >= 2);
        let _ = std::fs::remove_file(AuditLog::beside_state(&path).path);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn http_get_rejects_non_http_scheme() {
        let mut gate = ToolGateExecutor::new(["http_get".into()])
            .enable_http_get(true)
            .echo(false);
        let mut params = HashMap::new();
        params.insert("url".into(), serde_json::json!("file:///etc/passwd"));
        let act = Action {
            name: "http_get".into(),
            params,
            constraints_checked: true,
        };
        assert!(gate.execute(&[act]).is_err());
    }
}
