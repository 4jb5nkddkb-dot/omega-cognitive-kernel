use crate::error::Result;
use crate::types::{Observation, Prediction};
use std::collections::HashMap;

pub trait Predictor {
    fn predict(
        &self,
        observations: &[Observation],
        memory_ctx: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<Prediction>>;
}

pub struct ModelBasedPredictor;

impl Predictor for ModelBasedPredictor {
    fn predict(
        &self,
        observations: &[Observation],
        _memory_ctx: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<Prediction>> {
        let uncertainty = if observations.is_empty() {
            0.8
        } else {
            0.35 + (observations.len() as f64 * 0.02).min(0.4)
        };

        Ok(vec![Prediction {
            expected: serde_json::json!({"next": "stable"}),
            uncertainty,
            horizon_secs: 1.0,
        }])
    }
}
