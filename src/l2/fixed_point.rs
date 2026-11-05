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
    /// Soft nudge to RuleEffect.explore_bonus (will be sanitized after merge).
    pub explore_bonus_delta: f64,
    /// Soft nudge to RuleEffect.risk_tolerance (will be sanitized after merge).
    pub risk_tolerance_delta: f64,
}

pub struct OmegaDayanFixedPoint;

impl OmegaDayanFixedPoint {
    pub fn enter(&self, state: &SystemState) -> Result<FixedPointResult> {
        let score = self.align(
            &state.highest_attractor,
            state.generative_base.as_deref(),
            &state.active_code,
            state.avg_prediction_error,
            state.total_steps,
        );

        let mut modulation = Modulation::default();
        if score > 0.82 {
            modulation.bias_delta.insert("self_ref".into(), 0.18);
            modulation.bias_delta.insert("coherence".into(), 0.12);
            modulation.constraint_delta.insert("max_risk".into(), -0.04);
            modulation.bias_delta.insert("explore_preference".into(), -0.08);
            modulation.explore_bonus_delta = -0.06;
            modulation.risk_tolerance_delta = -0.03;
        } else {
            modulation
                .bias_delta
                .insert("observation_depth".into(), 0.14);
            modulation.constraint_delta.insert("max_risk".into(), 0.03);
            modulation.bias_delta.insert("explore_preference".into(), 0.06);
            modulation.explore_bonus_delta = 0.05;
            modulation.risk_tolerance_delta = 0.02;
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
        avg_error: f64,
        total_steps: u64,
    ) -> f64 {
        let mut score = 0.30;

        let omega_l = omega.to_lowercase();
        if omega_l.contains("self")
            || omega_l.contains("coherent")
            || omega_l.contains("omega")
            || omega_l.contains("dayan")
        {
            score += 0.18;
        }

        if let Some(base) = dayan {
            let base_l = base.to_lowercase();
            if base_l.contains("generative")
                || base_l.contains("cognitive_rules")
                || base_l.contains("dayan")
            {
                score += 0.14;
            }

            let omega_chars: std::collections::HashSet<char> = omega.chars().collect();
            let overlap = base.chars().filter(|c| omega_chars.contains(c)).count() as f64;
            let denom = omega.len().max(base.len()).max(1) as f64;
            score += (overlap / denom) * 0.18;
        }

        if active_code.contains_key("cognitive_rules") {
            score += 0.14;
            if let Some(code) = active_code.get("cognitive_rules") {
                let cl = code.to_lowercase();
                if cl.contains("self") || cl.contains("bias") || cl.contains("damp") {
                    score += 0.06;
                }
            }
        }

        if !active_code.is_empty() {
            score += 0.05;
        }

        if total_steps > 8 && avg_error < 0.35 {
            score += 0.08;
        } else if avg_error > 0.6 {
            score -= 0.06;
        }

        score.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SystemState;
    use std::collections::HashMap;

    fn state(omega: &str, dayan: Option<&str>, has_rules: bool, err: f64, steps: u64) -> SystemState {
        let mut active = HashMap::new();
        if has_rules {
            active.insert(
                "cognitive_rules".into(),
                "fn adjust_bias(err: f64) -> f64 { err * 0.8 } // self damp".into(),
            );
        }
        SystemState {
            active_code: active,
            highest_attractor: omega.into(),
            generative_base: dayan.map(|s| s.into()),
            last_trajectory_summary: None,
            avg_prediction_error: err,
            total_steps: steps,
        }
    }

    #[test]
    fn higher_score_when_aligned_keywords_present() {
        let fp = OmegaDayanFixedPoint;
        let s1 = state(
            "maintain coherent self-model omega",
            Some("dayan_generative_base"),
            true,
            0.2,
            20,
        );
        let s2 = state("random goal", None, false, 0.7, 2);
        let r1 = fp.enter(&s1).unwrap();
        let r2 = fp.enter(&s2).unwrap();
        assert!(r1.score > r2.score);
    }

    #[test]
    fn aligned_produces_negative_explore_nudge() {
        let fp = OmegaDayanFixedPoint;
        let s = state("coherent self omega", Some("dayan generative"), true, 0.2, 20);
        let r = fp.enter(&s).unwrap();
        if r.aligned {
            assert!(r.modulation.explore_bonus_delta <= 0.0);
        }
    }

    #[test]
    fn produces_modulation() {
        let fp = OmegaDayanFixedPoint;
        let s = state("coherent self", Some("cognitive_rules"), true, 0.25, 15);
        let r = fp.enter(&s).unwrap();
        assert!(!r.modulation.bias_delta.is_empty() || !r.modulation.constraint_delta.is_empty());
    }

    #[test]
    fn score_is_clamped() {
        let fp = OmegaDayanFixedPoint;
        let s = state(
            "self coherent generative omega dayan",
            Some("cognitive_rules generative dayan"),
            true,
            0.1,
            50,
        );
        let r = fp.enter(&s).unwrap();
        assert!(r.score >= 0.0 && r.score <= 1.0);
    }
}
