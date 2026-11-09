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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Action;
    use std::collections::HashMap;

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
}
