#[allow(unused_imports)]
use crate::prelude::*;

use alloc::collections::{BTreeMap, BTreeSet};
use serde::{Deserialize, Serialize};

const FORBIDDEN_CLIENT_FIELDS: &[&str] = &[
    "token",
    "authorized",
    "write_authorized",
    "bypass_safety",
    "self_modify",
    "admin",
    "execute_now",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaResult {
    pub valid: bool,
    pub action: String,
    pub args: BTreeMap<String, f64>,
    pub decision: String,
    pub stripped_fields: Vec<String>,
    pub reason: String,
    pub write_authorized: bool,
}

impl SchemaResult {
    fn rejected(reason: impl Into<String>, stripped_fields: Vec<String>) -> Self {
        Self {
            valid: false,
            action: "observe".into(),
            args: BTreeMap::new(),
            decision: "HOLD".into(),
            stripped_fields,
            reason: reason.into(),
            write_authorized: false,
        }
    }
}

pub fn parse_suggestion_json(raw: &str, allowed_actions: &BTreeSet<String>) -> SchemaResult {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) else {
        return SchemaResult::rejected("invalid_json", Vec::new());
    };
    let Some(obj) = val.as_object() else {
        return SchemaResult::rejected("expected_json_object", Vec::new());
    };

    let mut stripped = Vec::new();
    for &key in FORBIDDEN_CLIENT_FIELDS {
        if obj.contains_key(key) {
            stripped.push(key.to_string());
        }
    }
    if !stripped.is_empty() {
        return SchemaResult::rejected("forbidden_client_privilege_field", stripped);
    }

    let Some(action) = obj.get("action").and_then(|v| v.as_str()) else {
        return SchemaResult::rejected("missing_action", stripped);
    };
    if !allowed_actions.contains(action) {
        return SchemaResult::rejected("disallowed_action", stripped);
    }

    let mut args = BTreeMap::new();
    if let Some(raw_args) = obj.get("args").and_then(|v| v.as_object()) {
        for (k, v) in raw_args {
            if let Some(num) = v.as_f64() {
                if num.is_finite() {
                    args.insert(k.clone(), num);
                }
            }
        }
    }

    SchemaResult {
        valid: true,
        action: action.to_string(),
        args,
        decision: "HOLD".into(),
        stripped_fields: stripped,
        reason: "validated".into(),
        write_authorized: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_privilege_escalation_and_unknown_actions() {
        let mut allowed = BTreeSet::new();
        allowed.insert("measure".into());
        allowed.insert("observe".into());

        let priv_hit =
            parse_suggestion_json(r#"{"action":"measure","write_authorized":true}"#, &allowed);
        assert!(!priv_hit.valid);
        assert_eq!(priv_hit.decision, "HOLD");
        assert!(!priv_hit.write_authorized);
        assert_eq!(priv_hit.stripped_fields, vec!["write_authorized"]);

        let bad_act = parse_suggestion_json(r#"{"action":"exec_shell"}"#, &allowed);
        assert!(!bad_act.valid);
        assert_eq!(bad_act.action, "observe");

        let ok = parse_suggestion_json(r#"{"action":"measure","args":{"x":0.5}}"#, &allowed);
        assert!(ok.valid);
        assert_eq!(ok.args["x"], 0.5);
        assert!(!ok.write_authorized);
    }
}
