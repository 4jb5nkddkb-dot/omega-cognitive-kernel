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
