//! World executor: classic sandboxed actions + optional HTTP allowlist actions.

use crate::adapters::http::HttpToolAdapter;
use crate::error::{OmegaError, Result};
use crate::l0::executor::{Executor, SandboxedExecutor};
use crate::types::Action;

pub struct WorldExecutor {
    pub sandbox: SandboxedExecutor,
    pub http: Option<HttpToolAdapter>,
}

impl WorldExecutor {
    pub fn simulated(allowed: impl IntoIterator<Item = String>) -> Self {
        Self {
            sandbox: SandboxedExecutor::new(allowed),
            http: None,
        }
    }

    pub fn with_http(mut sandbox_allowed: Vec<String>, http: HttpToolAdapter) -> Self {
        for name in http.action_names() {
            if !sandbox_allowed.iter().any(|a| a == &name) {
                sandbox_allowed.push(name);
            }
        }
        Self {
            sandbox: SandboxedExecutor::new(sandbox_allowed),
            http: Some(http),
        }
    }
}

impl Executor for WorldExecutor {
    fn execute(&mut self, actions: &[Action]) -> Result<Vec<serde_json::Value>> {
        let mut results = Vec::new();
        for act in actions {
            if !act.constraints_checked {
                return Err(OmegaError::ExecutionBlocked(format!(
                    "{} (constraints not checked)",
                    act.name
                )));
            }
            if let Some(http) = self.http.as_mut() {
                if http.handles(&act.name) {
                    results.push(http.execute_named(&act.name)?);
                    continue;
                }
            }
            // Classic path: one action at a time through sandbox
            let one = self.sandbox.execute(std::slice::from_ref(act))?;
            results.extend(one);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::http::{HttpActionSpec, HttpAdapterConfig, HttpToolAdapter};
    use std::collections::HashMap;

    fn action(name: &str) -> Action {
        Action {
            name: name.into(),
            params: HashMap::new(),
            constraints_checked: true,
        }
    }

    #[test]
    fn classic_action_still_works_without_http() {
        let mut ex = WorldExecutor::simulated(vec!["observe".into()]);
        let res = ex.execute(&[action("observe")]).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0]["status"], "ok");
    }

    #[test]
    fn unknown_classic_blocked() {
        let mut ex = WorldExecutor::simulated(vec!["observe".into()]);
        assert!(ex.execute(&[action("delete_everything")]).is_err());
    }

    #[test]
    fn http_action_name_is_handled_flag() {
        let mut actions = HashMap::new();
        actions.insert(
            "probe_ok".into(),
            HttpActionSpec {
                method: "GET".into(),
                url: "https://httpbin.org/status/200".into(),
            },
        );
        let http = HttpToolAdapter::from_config(HttpAdapterConfig {
            timeout_ms: 500,
            max_body_bytes: 256,
            allowed_hosts: vec!["httpbin.org".into()],
            actions,
        })
        .unwrap();
        assert!(http.handles("probe_ok"));
        assert!(!http.handles("observe"));
    }
}
