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
        self.avg_prediction_error = (1.0 - alpha) * self.avg_prediction_error + alpha * mean_err;
        self.adaptation_score = (1.0 - self.avg_prediction_error).clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Trajectory;
    use std::collections::HashMap;

    fn traj_with_errors(errors: Vec<f64>) -> Trajectory {
        Trajectory {
            observations: vec![],
            predictions: vec![],
            actions: vec![],
            outcomes: vec![],
            prediction_errors: errors,
            meta: HashMap::new(),
        }
    }

    #[test]
    fn updates_step_count() {
        let mut cas = CasState::default();
        cas.update(&traj_with_errors(vec![0.2]));
        assert_eq!(cas.total_steps, 1);
        cas.update(&traj_with_errors(vec![0.1]));
        assert_eq!(cas.total_steps, 2);
    }

    #[test]
    fn adaptation_score_rises_with_low_error() {
        let mut cas = CasState::default();
        for _ in 0..20 {
            cas.update(&traj_with_errors(vec![0.05]));
        }
        assert!(cas.adaptation_score > 0.8);
        assert!(cas.avg_prediction_error < 0.2);
    }
}
