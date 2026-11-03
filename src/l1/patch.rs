use crate::types::{CognitivePatch, RuleEffect, Trajectory};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

pub struct PatchGenerator;

impl PatchGenerator {
    pub fn generate(traj: &Trajectory, abstraction_summary: &str) -> CognitivePatch {
        let high_error = traj.prediction_errors.iter().any(|&e| e > 0.45);
        let mean_err = if traj.prediction_errors.is_empty() {
            0.5
        } else {
            traj.prediction_errors.iter().sum::<f64>() / traj.prediction_errors.len() as f64
        };

        // Stronger damping when error is higher so subsequent uncertainty drops.
        let error_damping = if high_error {
            (0.35 + mean_err * 0.10).clamp(0.30, 0.55)
        } else {
            0.75
        };

        let mut action_preference = HashMap::new();
        if high_error {
            action_preference.insert("stabilize".into(), 1.5);
            action_preference.insert("observe".into(), 0.5);
            action_preference.insert("explore".into(), -0.3);
        } else {
            action_preference.insert("exploit".into(), 0.8);
            action_preference.insert("observe".into(), 0.2);
        }

        let risk = if high_error { 0.45 } else { 0.22 };

        let code = format!(
            r#"
// auto-generated cognitive rule (audit trail only — runtime uses RuleEffect)
// summary: {summary}
// error_damping: {damping:.3}
// generated_at: {ts}
fn adjust_bias(err: f64) -> f64 {{
    if err > 0.5 {{ {damping:.3} }} else {{ 1.0 }}
}}
"#,
            summary = abstraction_summary,
            damping = error_damping,
            ts = Utc::now().to_rfc3339()
        );

        CognitivePatch {
            patch_id: Uuid::new_v4(),
            description: abstraction_summary.to_string(),
            code,
            target_module: "cognitive_rules".into(),
            expected_effect: format!(
                "damp error by {:.2}, prefer {:?}",
                error_damping,
                action_preference.keys().collect::<Vec<_>>()
            ),
            risk_level: risk,
            created_at: Utc::now(),
            effect: RuleEffect {
                error_damping,
                action_preference,
                min_uncertainty: 0.08,
            },
        }
    }
}
