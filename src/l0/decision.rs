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

    fn extra_actions(memory_ctx: &HashMap<String, serde_json::Value>) -> Vec<String> {
        memory_ctx
            .get("available_actions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Map L2 meta-biases (self_ref, coherence, explore_preference, …)
    /// onto concrete action score deltas.
    fn l2_action_deltas(biases: &HashMap<String, f64>) -> HashMap<&'static str, f64> {
        let mut d = HashMap::new();
        let explore_pref = biases.get("explore_preference").copied().unwrap_or(0.0);
        let self_ref = biases.get("self_ref").copied().unwrap_or(0.0);
        let coherence = biases.get("coherence").copied().unwrap_or(0.0);
        let obs_depth = biases.get("observation_depth").copied().unwrap_or(0.0);

        *d.entry("explore").or_insert(0.0) += explore_pref * 1.4;
        *d.entry("exploit").or_insert(0.0) -= explore_pref * 0.4;
        *d.entry("stabilize").or_insert(0.0) -= explore_pref * 0.3;

        let meta_stable = (self_ref + coherence) * 0.7;
        *d.entry("stabilize").or_insert(0.0) += meta_stable;
        *d.entry("observe").or_insert(0.0) += meta_stable * 0.35;
        *d.entry("explore").or_insert(0.0) -= meta_stable * 0.25;

        *d.entry("observe").or_insert(0.0) += obs_depth * 1.1;

        d
    }

    /// Light priors for HTTP probe-style actions (only used if in candidate set).
    fn http_action_prior(name: &str, uncertainty: f64, explore_bonus: f64) -> f64 {
        match name {
            // Healthy probe: useful when we want external signal / mild explore
            "probe_ok" => 0.35 + explore_bonus * 0.25 + uncertainty * 0.15,
            // Failure probe: rare, only if strongly preferred via RuleEffect
            "probe_fail" => -0.2 + uncertainty * 0.05,
            // Slow probe: costly; discourage under high risk pressure (handled elsewhere)
            "probe_slow" => 0.05 + explore_bonus * 0.15,
            // Fetch structured metric: like observe+exploit hybrid
            "fetch_metric" => 0.4 + uncertainty * 0.1,
            _ => 0.2,
        }
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
        let extras = Self::extra_actions(memory_ctx);

        let max_risk = constraints.get("max_risk").copied().unwrap_or(0.65);
        let risk_pressure = (1.0 - max_risk).clamp(0.0, 0.9);

        let mut candidates: Vec<String> = ["observe", "explore", "exploit", "stabilize"]
            .into_iter()
            .map(str::to_string)
            .collect();
        for a in &extras {
            if !candidates.iter().any(|c| c == a) {
                candidates.push(a.clone());
            }
        }

        let mut best_name = candidates
            .first()
            .map(|s| s.as_str())
            .unwrap_or("observe");
        let mut best_score = f64::MIN;

        for name in &candidates {
            let mut score = *strat.get(name).unwrap_or(&1.0);
            score += effect.action_preference.get(name).copied().unwrap_or(0.0);
            score += intent.bias_from_memory.get(name).copied().unwrap_or(0.0);
            score += l2_deltas.get(name.as_str()).copied().unwrap_or(0.0);

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

            // External / HTTP-style actions
            if extras.iter().any(|e| e == name) {
                score += Self::http_action_prior(name, uncertainty, effect.explore_bonus);
                // Under tight max_risk, prefer not to hit the network
                score -= risk_pressure * 0.6;
                if name == "probe_slow" {
                    score -= risk_pressure * 0.5;
                }
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
                // When uncertain and network is available, mild boost to healthy probes
                if name == "probe_ok" || name == "fetch_metric" {
                    score += 0.35;
                }
            } else if name == "exploit" {
                score += 0.5;
            }

            if score > best_score {
                best_score = score;
                best_name = name.as_str();
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
        params.insert(
            "extra_actions".into(),
            serde_json::json!(extras),
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

    fn ctx_with_http() -> HashMap<String, serde_json::Value> {
        let mut ctx = ctx_default();
        ctx.insert(
            "available_actions".into(),
            serde_json::json!(["probe_ok", "probe_fail", "fetch_metric"]),
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

    #[test]
    fn strong_preference_can_select_probe_ok() {
        let dm = ConstrainedDecisionMaker;
        let intent = Intent {
            goal: "t".into(),
            priority: 1.0,
            bias_from_memory: HashMap::new(),
        };
        let mut effect = RuleEffect::default();
        effect
            .action_preference
            .insert("probe_ok".into(), 5.0);
        let mut ctx = ctx_with_http();
        ctx.insert(
            "cognitive_effect".into(),
            serde_json::to_value(effect).unwrap(),
        );
        let constraints = HashMap::new();
        let actions = dm
            .decide(&intent, &[pred(0.5)], &constraints, &ctx)
            .unwrap();
        assert_eq!(actions[0].name, "probe_ok");
    }

    #[test]
    fn without_available_actions_probe_not_chosen() {
        let dm = ConstrainedDecisionMaker;
        let intent = Intent {
            goal: "t".into(),
            priority: 1.0,
            bias_from_memory: HashMap::new(),
        };
        let mut effect = RuleEffect::default();
        // Preference alone is not enough if action is not in candidate set
        effect
            .action_preference
            .insert("probe_ok".into(), 5.0);
        let mut ctx = ctx_default();
        ctx.insert(
            "cognitive_effect".into(),
            serde_json::to_value(effect).unwrap(),
        );
        let constraints = HashMap::new();
        let actions = dm
            .decide(&intent, &[pred(0.5)], &constraints, &ctx)
            .unwrap();
        assert_ne!(actions[0].name, "probe_ok");
    }
}
