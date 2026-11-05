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

        if let Some(msg) = patch.effect.validate_bounds() {
            return Ok(ValidationReport {
                accepted: false,
                message: format!("effect rejected: {msg}"),
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
    use chrono::Utc;

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
        assert!(sandbox.validate(&patch).unwrap().accepted);
    }

    #[test]
    fn rejects_high_risk_patch() {
        let sandbox = CodeSandbox::default();
        let patch = make_patch("fn dangerous() {}", 0.9);
        assert!(!sandbox.validate(&patch).unwrap().accepted);
    }

    #[test]
    fn rejects_forbidden_keyword() {
        let sandbox = CodeSandbox::default();
        let patch = make_patch("use std::process::Command;", 0.1);
        assert!(!sandbox.validate(&patch).unwrap().accepted);
    }

    #[test]
    fn rejects_bad_effect_damping() {
        let sandbox = CodeSandbox::default();
        let mut patch = make_patch("fn ok() {}", 0.1);
        patch.effect.error_damping = 9.0;
        let report = sandbox.validate(&patch).unwrap();
        assert!(!report.accepted);
        assert!(report.message.contains("effect rejected"));
    }

    #[test]
    fn rejects_unknown_action_preference() {
        let sandbox = CodeSandbox::default();
        let mut patch = make_patch("fn ok() {}", 0.1);
        patch
            .effect
            .action_preference
            .insert("launch_missiles".into(), 1.0);
        assert!(!sandbox.validate(&patch).unwrap().accepted);
    }

    #[test]
    fn rejects_out_of_bounds_explore_bonus() {
        let sandbox = CodeSandbox::default();
        let mut patch = make_patch("fn ok() {}", 0.1);
        patch.effect.explore_bonus = 5.0;
        let report = sandbox.validate(&patch).unwrap();
        assert!(!report.accepted);
        assert!(report.message.contains("explore_bonus"));
    }

    #[test]
    fn rejects_out_of_bounds_risk_tolerance() {
        let sandbox = CodeSandbox::default();
        let mut patch = make_patch("fn ok() {}", 0.1);
        patch.effect.risk_tolerance = 0.01;
        let report = sandbox.validate(&patch).unwrap();
        assert!(!report.accepted);
        assert!(report.message.contains("risk_tolerance"));
    }

    #[test]
    fn sanitize_clamps_new_fields() {
        let mut effect = crate::types::RuleEffect {
            error_damping: 0.5,
            action_preference: Default::default(),
            min_uncertainty: 0.1,
            explore_bonus: 3.0,
            risk_tolerance: 0.05,
        };
        effect = effect.sanitize();
        assert!(effect.explore_bonus <= 1.5);
        assert!(effect.risk_tolerance >= 0.15);
    }
}
