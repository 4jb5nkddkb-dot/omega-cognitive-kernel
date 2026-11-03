use crate::error::Result;
use crate::types::{Action, Intent, Prediction};
use std::collections::HashMap;

pub trait DecisionMaker {
    fn decide(
        &self,
        intent: &Intent,
        predictions: &[Prediction],
        constraints: &HashMap<String, f64>,
    ) -> Result<Vec<Action>>;
}

pub struct ConstrainedDecisionMaker;

impl DecisionMaker for ConstrainedDecisionMaker {
    fn decide(
        &self,
        intent: &Intent,
        predictions: &[Prediction],
        _constraints: &HashMap<String, f64>,
    ) -> Result<Vec<Action>> {
        let mut params = HashMap::new();
        params.insert("goal".into(), serde_json::Value::String(intent.goal.clone()));
        if let Some(p) = predictions.first() {
            params.insert("uncertainty".into(), serde_json::json!(p.uncertainty));
        }

        Ok(vec![Action {
            name: "observe".into(),
            params,
            constraints_checked: true,
        }])
    }
}
