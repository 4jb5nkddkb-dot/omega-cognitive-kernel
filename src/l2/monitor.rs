use crate::error::Result;
use crate::types::{MetaMetrics, SystemState};
use chrono::Utc;

pub struct MetaMonitor;

impl MetaMonitor {
    pub fn evaluate(&self, state: &SystemState) -> Result<MetaMetrics> {
        let complexity =
            (state.active_code.len() as f64 * 0.12) + (state.total_steps as f64 * 0.01);

        // Data-driven consistency: lower when prediction error is high or volatile.
        // Use a soft inverse of avg_prediction_error + presence of generative base.
        let err = state.avg_prediction_error.clamp(0.0, 1.0);
        let mut consistency = (1.0 - err * 0.85).clamp(0.25, 0.95);
        if state.generative_base.is_some() {
            consistency = (consistency + 0.08).min(0.95);
        }
        if state.active_code.contains_key("cognitive_rules") {
            consistency = (consistency + 0.05).min(0.95);
        }

        let predictive_power = (1.0 - state.avg_prediction_error).clamp(0.2, 0.95);

        let self_ref_stability = if state.generative_base.is_some() {
            0.58 + (state.active_code.len() as f64 * 0.03).min(0.3)
        } else {
            0.32
        };

        Ok(MetaMetrics {
            complexity,
            consistency,
            predictive_power,
            self_ref_stability,
            timestamp: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn state(err: f64, has_base: bool, has_rules: bool) -> SystemState {
        let mut active = HashMap::new();
        if has_rules {
            active.insert("cognitive_rules".into(), "fn x() {}".into());
        }
        SystemState {
            active_code: active,
            highest_attractor: "test".into(),
            generative_base: if has_base {
                Some("dayan".into())
            } else {
                None
            },
            last_trajectory_summary: None,
            avg_prediction_error: err,
            total_steps: 5,
        }
    }

    #[test]
    fn higher_error_lowers_consistency() {
        let m = MetaMonitor;
        let high = m.evaluate(&state(0.8, false, false)).unwrap();
        let low = m.evaluate(&state(0.1, false, false)).unwrap();
        assert!(low.consistency > high.consistency);
    }

    #[test]
    fn generative_base_boosts_consistency() {
        let m = MetaMonitor;
        let with = m.evaluate(&state(0.3, true, true)).unwrap();
        let without = m.evaluate(&state(0.3, false, false)).unwrap();
        assert!(with.consistency > without.consistency);
    }
}
