use crate::error::Result;
use crate::types::{Observation, Prediction, RuleEffect};
use std::collections::HashMap;

pub trait Predictor {
    fn predict(
        &self,
        observations: &[Observation],
        memory_ctx: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<Prediction>>;
}

pub struct ModelBasedPredictor;

impl ModelBasedPredictor {
    fn effect_from_ctx(memory_ctx: &HashMap<String, serde_json::Value>) -> RuleEffect {
        memory_ctx
            .get("cognitive_effect")
            .and_then(|v| serde_json::from_value::<RuleEffect>(v.clone()).ok())
            .unwrap_or_default()
    }

    fn strategy_confidence(memory_ctx: &HashMap<String, serde_json::Value>) -> f64 {
        let Some(strategies) = memory_ctx.get("strategies").and_then(|v| v.as_array()) else {
            return 0.0;
        };
        if strategies.is_empty() {
            return 0.0;
        }
        let best = strategies
            .iter()
            .filter_map(|s| s.get("weight").and_then(|w| w.as_f64()))
            .fold(0.0_f64, f64::max);
        ((best - 1.0) / 4.0).clamp(0.0, 1.0)
    }

    fn recent_case_error(memory_ctx: &HashMap<String, serde_json::Value>) -> f64 {
        memory_ctx
            .get("cases")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("error_mean"))
            .and_then(|e| e.as_f64())
            .unwrap_or(0.5)
    }
}

impl Predictor for ModelBasedPredictor {
    fn predict(
        &self,
        observations: &[Observation],
        memory_ctx: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<Prediction>> {
        let effect = Self::effect_from_ctx(memory_ctx);
        let strat_conf = Self::strategy_confidence(memory_ctx);
        let case_err = Self::recent_case_error(memory_ctx);

        // Base uncertainty: starts elevated, reduced by strategy confidence.
        let mut uncertainty = if observations.is_empty() {
            0.85
        } else {
            0.55 + case_err * 0.35 - strat_conf * 0.25
        };
        uncertainty = uncertainty.clamp(0.15, 0.95);

        // S1 consequence: committed RuleEffect actually lowers uncertainty.
        uncertainty = (uncertainty * effect.error_damping).max(effect.min_uncertainty);

        Ok(vec![Prediction {
            expected: serde_json::json!({
                "next": "stable",
                "strategy_confidence": strat_conf,
                "damping": effect.error_damping
            }),
            uncertainty,
            horizon_secs: 1.0,
        }])
    }
}
