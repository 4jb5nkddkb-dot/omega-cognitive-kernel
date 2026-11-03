use crate::error::Result;
use crate::types::{Action, Intent, Prediction, RuleEffect};
use std::collections::HashMap;

pub trait DecisionMaker {
    fn decide(
        &self,
        intent: &Intent,
        predictions: &[Prediction],
        constraints: &HashMap<String, f64>,
        memory_ctx: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<Action>>;
}

pub struct ConstrainedDecisionMaker;

impl ConstrainedDecisionMaker {
    fn effect_from_ctx(memory_ctx: &HashMap<String, serde_json::Value>) -> RuleEffect {
        memory_ctx
            .get("cognitive_effect")
            .and_then(|v| serde_json::from_value::<RuleEffect>(v.clone()).ok())
            .unwrap_or_default()
    }

    fn strategy_weights(memory_ctx: &HashMap<String, serde_json::Value>) -> HashMap<String, f64> {
        let mut out = HashMap::new();
        if let Some(arr) = memory_ctx.get("strategies").and_then(|v| v.as_array()) {
            for s in arr {
                if let (Some(name), Some(w)) = (
                    s.get("name").and_then(|n| n.as_str()),
                    s.get("weight").and_then(|w| w.as_f64()),
                ) {
                    out.insert(name.to_string(), w);
                }
            }
        }
        out
    }
}

impl DecisionMaker for ConstrainedDecisionMaker {
    fn decide(
        &self,
        intent: &Intent,
        predictions: &[Prediction],
        _constraints: &HashMap<String, f64>,
        memory_ctx: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<Action>> {
        let effect = Self::effect_from_ctx(memory_ctx);
        let strat = Self::strategy_weights(memory_ctx);
        let uncertainty = predictions.first().map(|p| p.uncertainty).unwrap_or(0.5);

        // Candidate actions (must stay inside executor whitelist).
        let candidates = ["observe", "explore", "exploit", "stabilize"];

        let mut best_name = "observe";
        let mut best_score = f64::MIN;

        for name in candidates {
            let mut score = *strat.get(name).unwrap_or(&1.0);
            // RuleEffect preference (from committed patches).
            score += effect.action_preference.get(name).copied().unwrap_or(0.0);
            // Intent / L2 biases.
            score += intent.bias_from_memory.get(name).copied().unwrap_or(0.0);
            // Heuristic: high uncertainty → prefer stabilize/observe.
            if uncertainty > 0.45 {
                if name == "stabilize" {
                    score += 0.8;
                }
                if name == "observe" {
                    score += 0.3;
                }
                if name == "explore" {
                    score -= 0.2;
                }
            } else if name == "exploit" {
                score += 0.5;
            }
            if score > best_score {
                best_score = score;
                best_name = name;
            }
        }

        let mut params = HashMap::new();
        params.insert(
            "goal".into(),
            serde_json::Value::String(intent.goal.clone()),
        );
        params.insert("uncertainty".into(), serde_json::json!(uncertainty));
        params.insert("score".into(), serde_json::json!(best_score));
        params.insert("damping".into(), serde_json::json!(effect.error_damping));

        Ok(vec![Action {
            name: best_name.into(),
            params,
            constraints_checked: true,
        }])
    }
}
