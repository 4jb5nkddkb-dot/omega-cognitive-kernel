use crate::types::Trajectory;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CasState {
    pub total_steps: u64,
    pub avg_prediction_error: f64,
    pub adaptation_score: f64,
}

impl CasState {
    pub fn update(&mut self, traj: &Trajectory) {
        self.total_steps += 1;
        let mean_err = if traj.prediction_errors.is_empty() {
            0.0
        } else {
            traj.prediction_errors.iter().sum::<f64>() / traj.prediction_errors.len() as f64
        };

        let alpha = 0.1;
        self.avg_prediction_error =
            (1.0 - alpha) * self.avg_prediction_error + alpha * mean_err;
        self.adaptation_score = (1.0 - self.avg_prediction_error).clamp(0.0, 1.0);
    }
}
