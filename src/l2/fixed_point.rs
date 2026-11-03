use crate::error::Result;
use crate::types::SystemState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedPointResult {
    pub mode: String,
    pub aligned: bool,
    pub score: f64,
    pub modulation: Modulation,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Modulation {
    pub bias_delta: HashMap<String, f64>,
    pub constraint_delta: HashMap<String, f64>,
}

pub struct OmegaDayanFixedPoint;

impl OmegaDayanFixedPoint {
    /// Enter "observation = observed" mode and test whether
    /// omega (highest attractor) and dayan (generative base) converge.
    pub fn enter(&self, state: &SystemState) -> Result<FixedPointResult> {
        let score = self.align(
            &state.highest_attractor,
            state.generative_base.as_deref(),
            &state.active_code,
        );

        let mut modulation = Modulation::default();
        if score > 0.82 {
            modulation.bias_delta.insert("self_ref".into(), 0.18);
            modulation.bias_delta.insert("coherence".into(), 0.12);
            modulation
                .constraint_delta
                .insert("max_risk".into(), -0.04);
        } else {
            modulation
                .bias_delta
                .insert("observation_depth".into(), 0.14);
            modulation.constraint_delta.insert("max_risk".into(), 0.03);
        }

        Ok(FixedPointResult {
            mode: "observation_equals_observed".into(),
            aligned: score > 0.82,
            score,
            modulation,
        })
    }

    fn align(
        &self,
        omega: &str,
        dayan: Option<&str>,
        active_code: &HashMap<String, String>,
    ) -> f64 {
        let mut score = 0.38;

        if let Some(base) = dayan {
            if omega.to_lowercase().contains("self")
                || omega.to_lowercase().contains("coherent")
                || base.contains("generative")
                || base.contains("cognitive_rules")
            {
                score += 0.22;
            }

            let omega_chars: std::collections::HashSet<char> = omega.chars().collect();
            let overlap = base
                .chars()
                .filter(|c| omega_chars.contains(c))
                .count() as f64;
            let denom = omega.len().max(base.len()).max(1) as f64;
            score += (overlap / denom) * 0.25;
        }

        if active_code.contains_key("cognitive_rules") {
            score += 0.18;
        }

        if active_code.len() >= 1 {
            score += 0.08;
        }

        score.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SystemState;
    use std::collections::HashMap;

    fn state(omega: &str, dayan: Option<&str>, has_rules: bool) -> SystemState {
        let mut active = HashMap::new();
        if has_rules {
            active.insert("cognitive_rules".into(), "fn x() {}".into());
        }
        SystemState {
            active_code: active,
            highest_attractor: omega.into(),
            generative_base: dayan.map(|s| s.into()),
            last_trajectory_summary: None,
            avg_prediction_error: 0.3,
            total_steps: 10,
        }
    }

    #[test]
    fn higher_score_when_aligned_keywords_present() {
        let fp = OmegaDayanFixedPoint;
        let s1 = state("maintain coherent self-model", Some("dayan_generative_base"), true);
        let s2 = state("random goal", None, false);
        let r1 = fp.enter(&s1).unwrap();
        let r2 = fp.enter(&s2).unwrap();
        assert!(r1.score > r2.score);
    }

    #[test]
    fn produces_modulation() {
        let fp = OmegaDayanFixedPoint;
        let s = state("coherent self", Some("cognitive_rules"), true);
        let r = fp.enter(&s).unwrap();
        assert!(!r.modulation.bias_delta.is_empty() || !r.modulation.constraint_delta.is_empty());
    }

    #[test]
    fn score_is_clamped() {
        let fp = OmegaDayanFixedPoint;
        let s = state("self coherent generative", Some("cognitive_rules generative"), true);
        let r = fp.enter(&s).unwrap();
        assert!(r.score >= 0.0 && r.score <= 1.0);
    }
}
