use crate::audit::{AuditEvent, AuditLog};
use crate::error::{OmegaError, Result};
use crate::types::Action;
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

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
///
/// Safe first real adapter: the kernel can decide, you can inspect what it
/// would have done, without performing dangerous side effects.
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

/// Tool-call gate: allow-list + explicit deny-list, optional audit side-channel.
///
/// Product story: a model (or agent) *proposes* tool names; this executor is the
/// last line of defense before side effects. Denied tools never run; they are
/// recorded when `audit` is set. No real network/FS side effects in this type.
pub struct ToolGateExecutor {
    allowed: HashSet<String>,
    denied: HashSet<String>,
    audit: Option<AuditLog>,
    echo: bool,
    /// If true, denied tools return Err and stop the batch; if false, skip and continue.
    fail_closed: bool,
}

impl ToolGateExecutor {
    pub fn new(allowed: impl IntoIterator<Item = String>) -> Self {
        Self {
            allowed: allowed.into_iter().collect(),
            denied: HashSet::new(),
            audit: None,
            echo: true,
            fail_closed: true,
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
            results.push(serde_json::json!({
                "status": "ok",
                "action": act.name,
                "params": act.params,
                // Simulated — no real side effect.
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
        let sum = AuditLog::beside_state(&path).summarize().unwrap();
        // ToolAllowed + ToolDenied counted in total_events via read
        let rows = AuditLog::beside_state(&path).read_all().unwrap();
        assert!(rows.len() >= 2);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(AuditLog::beside_state(&path).path);
        let _ = sum;
    }
}
