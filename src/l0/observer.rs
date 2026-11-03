use crate::error::Result;
use crate::types::{Modality, Observation};
use chrono::Utc;

pub trait Observer {
    fn observe(&mut self) -> Result<Vec<Observation>>;
}

pub struct MultiModalObserver {
    pub step_counter: u64,
}

impl MultiModalObserver {
    pub fn new() -> Self {
        Self { step_counter: 0 }
    }
}

impl Default for MultiModalObserver {
    fn default() -> Self {
        Self::new()
    }
}

impl Observer for MultiModalObserver {
    fn observe(&mut self) -> Result<Vec<Observation>> {
        self.step_counter += 1;
        Ok(vec![Observation {
            timestamp: Utc::now(),
            modality: Modality::Structured,
            content: serde_json::json!({
                "step": self.step_counter,
                "signal": "world_tick",
                "value": (self.step_counter as f64 * 0.1).sin()
            }),
            confidence: 0.95,
            source: "sim_sensor".into(),
        }])
    }
}
