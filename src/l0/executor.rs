use crate::error::{OmegaError, Result};
use crate::types::Action;
use std::collections::HashSet;

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
}
