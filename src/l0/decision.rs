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

    /// Map L2 meta-biases (self_ref, coherence, explore_preference, …)
    /// onto concrete action score deltas.
    fn l2_action_deltas(biases: &HashMap<String, f64>) -> HashMap<&'static str, f64> {
        let mut d = HashMap::new();
        let explore_pref = biases.get("explore_preference").copied().unwrap_or(0.0);
        let self_ref = biases.get("self_ref").copied().unwrap_or(0.0);
        let coherence = biases.get("coherence").copied().unwrap_or(0.0);
        let obs_depth = biases.get("observation_depth").copied().unwrap_or(0.0);

        // explore_preference: direct drive for explore vs exploit/stabilize
        *d.entry("explore").or_insert(0.0) += explore_pref * 1.4;
        *d.entry("exploit").or_insert(0.0) -= explore_pref * 0.4;
        *d.entry("stabilize").or_insert(0.0) -= explore_pref * 0.3;

        // self_ref / coherence → prefer stabilize + mild observe
        let meta_stable = (self_ref + coherence) * 0.7;
        *d.entry("stabilize").or_insert(0.0) += meta_stable;
        *d.entry("observe").or_insert(0.0) += meta_stable * 0.35;
        *d.entry("explore").or_insert(0.0) -= meta_stable * 0.25;

        // observation_depth → observe
        *d.entry("observe").or_insert(0.0) += obs_depth * 1.1;

        d
    }
}

impl DecisionMaker for ConstrainedDecisionMaker {
    fn decide(
        &self,
        intent: &Intent,
        predictions: &[Prediction],
        constraints: &HashMap<String, f64>,
        memory_ctx: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<Action>> {
        let effect = Self::effect_from_ctx(memory_ctx);
        let strat = Self::strategy_weights(memory_ctx);
        let uncertainty = predictions.first().map(|p| p.uncertainty).unwrap_or(0.5);
        let l2_deltas = Self::l2_action_deltas(&intent.bias_from_memory);

        let max_risk = constraints.get("max_risk").copied().unwrap_or(0.65);
        let risk_pressure = (1.0 - max_risk).clamp(0.0, 0.9);

        let candidates = ["observe", "explore", "exploit", "stabilize"];

        let mut best_name = "observe";
        let mut best_score = f64::MIN;

        for name in candidates {
            let mut score = *strat.get(name).unwrap_or(&1.0);
            // L1 RuleEffect preferences
            score += effect.action_preference.get(name).copied().unwrap_or(0.0);
            // Direct action-named biases from memory (if any)
            score += intent.bias_from_memory.get(name).copied().unwrap_or(0.0);
            // L2 meta-bias mapping (self_ref / coherence / explore_preference / …)
            score += l2_deltas.get(name).copied().unwrap_or(0.0);

            if name == "explore" {
                score += effect.explore_bonus;
                score -= risk_pressure * 1.2;
            }
            if name == "stabilize" {
                score += risk_pressure * 0.9;
            }
            if name == "observe" {
                score += risk_pressure * 0.25;
            }

            let high_unc_threshold = effect.risk_tolerance;
            if uncertainty > high_unc_threshold {
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
        params.insert(
            "explore_bonus".into(),
            serde_json::json!(effect.explore_bonus),
        );
        params.insert(
            "risk_tolerance".into(),
            serde_json::json!(effect.risk_tolerance),
        );
        params.insert("max_risk".into(), serde_json::json!(max_risk));
        params.insert(
            "l2_explore_pref".into(),
            serde_json::json!(intent
                .bias_from_memory
                .get("explore_preference")
                .copied()
                .unwrap_or(0.0)),
        );

        Ok(vec![Action {
            name: best_name.into(),
            params,
            constraints_checked: true,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Prediction;

    fn pred(unc: f64) -> Prediction {
        Prediction {
            expected: serde_json::json!({}),
            uncertainty: unc,
            horizon_secs: 1.0,
        }
    }

    fn ctx_default() -> HashMap<String, serde_json::Value> {
        let mut ctx = HashMap::new();
        ctx.insert(
            "cognitive_effect".into(),
            serde_json::to_value(RuleEffect::default()).unwrap(),
        );
        ctx
    }

    #[test]
    fn tight_max_risk_prefers_stabilize_over_explore() {
        let dm = ConstrainedDecisionMaker;
        let intent = Intent {
            goal: "t".into(),
            priority: 1.0,
            bias_from_memory: HashMap::new(),
        };
        let mut constraints = HashMap::new();
        constraints.insert("max_risk".into(), 0.25);
        let actions = dm
            .decide(&intent, &[pred(0.4)], &constraints, &ctx_default())
            .unwrap();
        assert_ne!(actions[0].name, "explore");
    }

    #[test]
    fn l2_explore_preference_bias_boosts_explore() {
        let dm = ConstrainedDecisionMaker;
        let mut biases = HashMap::new();
        biases.insert("explore_preference".into(), 1.5);
        let intent = Intent {
            goal: "t".into(),
            priority: 1.0,
            bias_from_memory: biases,
        };
        let constraints = HashMap::new();
        let actions = dm
            .decide(&intent, &[pred(0.3)], &constraints, &ctx_default())
            .unwrap();
        assert_eq!(actions[0].name, "explore");
    }

    #[test]
    fn l2_self_ref_bias_boosts_stabilize() {
        let dm = ConstrainedDecisionMaker;
        let mut biases = HashMap::new();
        biases.insert("self_ref".into(), 2.0);
        biases.insert("coherence".into(), 1.5);
        let intent = Intent {
            goal: "t".into(),
            priority: 1.0,
            bias_from_memory: biases,
        };
        let constraints = HashMap::new();
        let actions = dm
            .decide(&intent, &[pred(0.3)], &constraints, &ctx_default())
            .unwrap();
        assert_eq!(actions[0].name, "stabilize");
    }
}
