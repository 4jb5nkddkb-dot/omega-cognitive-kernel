use crate::error::Result;
use crate::types::CognitivePatch;
use uuid::Uuid;

#[derive(Debug)]
pub struct ValidationReport {
    pub accepted: bool,
    pub message: String,
    pub patch_id: Uuid,
}

pub struct CodeSandbox {
    pub max_risk: f64,
    pub forbidden_keywords: Vec<String>,
}

impl Default for CodeSandbox {
    fn default() -> Self {
        Self {
            max_risk: 0.65,
            forbidden_keywords: vec![
                "std::process".into(),
                "std::fs".into(),
                "unsafe".into(),
                "include!".into(),
                "Command::".into(),
            ],
        }
    }
}

impl CodeSandbox {
    pub fn validate(&self, patch: &CognitivePatch) -> Result<ValidationReport> {
        if patch.risk_level > self.max_risk {
            return Ok(ValidationReport {
                accepted: false,
                message: format!(
                    "risk {:.2} exceeds max_risk {:.2}",
                    patch.risk_level, self.max_risk
                ),
                patch_id: patch.patch_id,
            });
        }

        for kw in &self.forbidden_keywords {
            if patch.code.contains(kw) {
                return Ok(ValidationReport {
                    accepted: false,
                    message: format!("forbidden keyword detected: {}", kw),
                    patch_id: patch.patch_id,
                });
            }
        }

        if patch.code.len() > 12_000 {
            return Ok(ValidationReport {
                accepted: false,
                message: "code too long".into(),
                patch_id: patch.patch_id,
            });
        }

        Ok(ValidationReport {
            accepted: true,
            message: "accepted".into(),
            patch_id: patch.patch_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CognitivePatch;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_patch(code: &str, risk: f64) -> CognitivePatch {
        CognitivePatch {
            patch_id: Uuid::new_v4(),
            description: "test".into(),
            code: code.into(),
            target_module: "cognitive_rules".into(),
            expected_effect: "test".into(),
            risk_level: risk,
            created_at: Utc::now(),
            effect: crate::types::RuleEffect::default(),
        }
    }

    #[test]
    fn accepts_safe_low_risk_patch() {
        let sandbox = CodeSandbox::default();
        let patch = make_patch("fn adjust_bias(err: f64) -> f64 { err * 0.9 }", 0.2);
        let report = sandbox.validate(&patch).unwrap();
        assert!(report.accepted);
        assert_eq!(report.message, "accepted");
    }

    #[test]
    fn rejects_high_risk_patch() {
        let sandbox = CodeSandbox::default();
        let patch = make_patch("fn dangerous() {}", 0.9);
        let report = sandbox.validate(&patch).unwrap();
        assert!(!report.accepted);
        assert!(report.message.contains("risk"));
    }

    #[test]
    fn rejects_forbidden_keyword() {
        let sandbox = CodeSandbox::default();
        let patch = make_patch("use std::process::Command;", 0.1);
        let report = sandbox.validate(&patch).unwrap();
        assert!(!report.accepted);
        assert!(report.message.contains("forbidden keyword"));
    }

    #[test]
    fn rejects_oversized_code() {
        let sandbox = CodeSandbox::default();
        let huge = "a".repeat(13_000);
        let patch = make_patch(&huge, 0.1);
        let report = sandbox.validate(&patch).unwrap();
        assert!(!report.accepted);
        assert_eq!(report.message, "code too long");
    }
}
