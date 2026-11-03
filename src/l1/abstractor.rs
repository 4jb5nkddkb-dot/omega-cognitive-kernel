use crate::error::Result;
use crate::types::Trajectory;

#[derive(Debug)]
pub struct Abstraction {
    pub summary: String,
    pub mean_error: f64,
    pub action_count: usize,
}

pub struct Abstractor;

impl Abstractor {
    pub fn distill(&self, traj: &Trajectory) -> Result<Abstraction> {
        let mean_error = if traj.prediction_errors.is_empty() {
            0.0
        } else {
            traj.prediction_errors.iter().sum::<f64>() / traj.prediction_errors.len() as f64
        };

        Ok(Abstraction {
            summary: format!(
                "obs={} mean_err={:.3} actions={}",
                traj.observations.len(),
                mean_error,
                traj.actions.len()
            ),
            mean_error,
            action_count: traj.actions.len(),
        })
    }
}
