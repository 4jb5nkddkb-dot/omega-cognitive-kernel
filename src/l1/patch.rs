use crate::types::{CognitivePatch, Trajectory};
use chrono::Utc;
use uuid::Uuid;

pub struct PatchGenerator;

impl PatchGenerator {
    pub fn generate(traj: &Trajectory, abstraction_summary: &str) -> CognitivePatch {
        let risk = if traj.prediction_errors.iter().any(|&e| e > 0.7) {
            0.55
        } else {
            0.22
        };

        let code = format!(
            r#"
// auto-generated cognitive rule
// summary: {}
// generated_at: {}
fn adjust_bias(err: f64) -> f64 {{
    if err > 0.5 {{ 0.85 }} else {{ 1.08 }}
}}
"#,
            abstraction_summary,
            Utc::now().to_rfc3339()
        );

        CognitivePatch {
            patch_id: Uuid::new_v4(),
            description: abstraction_summary.to_string(),
            code,
            target_module: "cognitive_rules".into(),
            expected_effect: "reduce future prediction error".into(),
            risk_level: risk,
            created_at: Utc::now(),
        }
    }
}
