use crate::types::{CognitivePatch, RuleEffect, Trajectory};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

/// Optional context the generator can use to make more targeted effects.
#[derive(Debug, Clone, Default)]
pub struct PatchContext {
    /// Recent mean prediction error (from case / cas).
    pub recent_mean_error: f64,
    /// Strategy weights (name → weight).
    pub strategy_weights: HashMap<String, f64>,
    /// How many recent cases had high error.
    pub high_error_streak: u32,
}

pub struct PatchGenerator;

impl PatchGenerator {
    pub fn generate(traj: &Trajectory, abstraction_summary: &str) -> CognitivePatch {
        Self::generate_with_context(traj, abstraction_summary, &PatchContext::default())
    }

    pub fn generate_with_context(
        traj: &Trajectory,
        abstraction_summary: &str,
        ctx: &PatchContext,
    ) -> CognitivePatch {
        let mean_err = if traj.prediction_errors.is_empty() {
            ctx.recent_mean_error.max(0.5)
        } else {
            traj.prediction_errors.iter().sum::<f64>() / traj.prediction_errors.len() as f64
        };
        let high_error = mean_err > 0.42 || ctx.high_error_streak >= 2;

        // Stronger damping when error is higher so subsequent uncertainty drops.
        let error_damping = if high_error {
            (0.32 + mean_err * 0.12).clamp(0.28, 0.55)
        } else {
            0.78
        };

        let mut action_preference = HashMap::new();

        let (explore_bonus, risk_tolerance) = if high_error {
            // Prefer stabilize / observe; discourage pure explore.
            action_preference.insert("stabilize".into(), 1.6);
            action_preference.insert("observe".into(), 0.6);
            action_preference.insert("explore".into(), -0.4);
            (-0.35, 0.38) // treat uncertainty as high more readily
        } else {
            // Low error: lean exploit, mild explore if strategies are weak.
            action_preference.insert("exploit".into(), 0.9);
            action_preference.insert("observe".into(), 0.25);

            let best_strat = ctx
                .strategy_weights
                .values()
                .cloned()
                .fold(1.0_f64, f64::max);
            if best_strat < 1.8 {
                // Strategies not yet confident → allow a little exploration.
                action_preference.insert("explore".into(), 0.4);
                (0.25, 0.52)
            } else {
                (-0.1, 0.48)
            }
        };

        // If a strategy already has high weight, reinforce it slightly.
        if let Some((name, w)) = ctx
            .strategy_weights
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        {
            if *w > 2.0 {
                let boost = action_preference.entry(name.clone()).or_insert(0.0);
                *boost += 0.35;
            }
        }

        let risk = if high_error { 0.48 } else { 0.20 };

        let code = format!(
            r#"
// auto-generated cognitive rule (audit trail only — runtime uses RuleEffect)
// summary: {summary}
// error_damping: {damping:.3}  explore_bonus: {eb:.2}  risk_tolerance: {rt:.2}
// generated_at: {ts}
fn adjust_bias(err: f64) -> f64 {{
    if err > 0.5 {{ {damping:.3} }} else {{ 1.0 }}
}}
"#,
            summary = abstraction_summary,
            damping = error_damping,
            eb = explore_bonus,
            rt = risk_tolerance,
            ts = Utc::now().to_rfc3339()
        );

        CognitivePatch {
            patch_id: Uuid::new_v4(),
            description: abstraction_summary.to_string(),
            code,
            target_module: "cognitive_rules".into(),
            expected_effect: format!(
                "damp={:.2} explore_bonus={:.2} risk_tol={:.2} prefs={:?}",
                error_damping,
                explore_bonus,
                risk_tolerance,
                action_preference.keys().collect::<Vec<_>>()
            ),
            risk_level: risk,
            created_at: Utc::now(),
            effect: RuleEffect {
                error_damping,
                action_preference,
                min_uncertainty: 0.08,
                explore_bonus,
                risk_tolerance,
            },
        }
    }
}
