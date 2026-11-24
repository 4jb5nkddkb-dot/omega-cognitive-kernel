#[allow(unused_imports)]
use crate::prelude::*;

use alloc::collections::BTreeSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmProposal {
    pub action: String,
    pub decision: String,
    pub rationale: String,
    pub write_authorized: bool,
}

#[derive(Debug, Clone)]
pub struct LlmBridge {
    pub allowed_actions: BTreeSet<String>,
}

impl LlmBridge {
    pub fn new<I, S>(actions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            allowed_actions: actions.into_iter().map(Into::into).collect(),
        }
    }

    pub fn validate_proposal(&self, action: &str, rationale: &str) -> LlmProposal {
        let permitted = self.allowed_actions.contains(action);
        LlmProposal {
            action: if permitted {
                action.to_string()
            } else {
                "observe".into()
            },
            decision: "HOLD".into(),
            rationale: rationale.to_string(),
            write_authorized: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_bridge_forces_hold_and_filters_unknown_actions() {
        let bridge = LlmBridge::new(["measure", "observe"]);
        let blocked = bridge.validate_proposal("rm_rf", "unsafe");
        assert_eq!(blocked.action, "observe");
        assert_eq!(blocked.decision, "HOLD");
        assert!(!blocked.write_authorized);
    }
}
