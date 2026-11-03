use crate::error::Result;
use crate::types::{MetaMetrics, SystemState};
use chrono::Utc;

pub struct MetaMonitor;

impl MetaMonitor {
    pub fn evaluate(&self, state: &SystemState) -> Result<MetaMetrics> {
        let complexity = (state.active_code.len() as f64 * 0.12) + (state.total_steps as f64 * 0.01);
        let consistency = 0.72;
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
