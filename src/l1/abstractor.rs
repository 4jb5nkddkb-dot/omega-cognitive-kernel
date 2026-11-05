use crate::error::Result;
use crate::types::Trajectory;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Abstraction {
    pub summary: String,
    pub mean_error: f64,
    pub error_variance: f64,
    pub action_count: usize,
    /// Fraction of actions that look "successful" (low following error or non-noop).
    pub success_rate: f64,
    /// Action name → count.
    pub action_dist: HashMap<String, usize>,
}

pub struct Abstractor;

impl Abstractor {
    pub fn distill(&self, traj: &Trajectory) -> Result<Abstraction> {
        let n = traj.prediction_errors.len();
        let mean_error = if n == 0 {
            0.0
        } else {
            traj.prediction_errors.iter().sum::<f64>() / n as f64
        };

        let error_variance = if n < 2 {
            0.0
        } else {
            let mean = mean_error;
            traj.prediction_errors
                .iter()
                .map(|e| (e - mean).powi(2))
                .sum::<f64>()
                / (n - 1) as f64
        };

        let mut action_dist: HashMap<String, usize> = HashMap::new();
        for a in &traj.actions {
            *action_dist.entry(a.name.clone()).or_insert(0) += 1;
        }

        // Heuristic success: later errors lower than mean, or action is stabilize/exploit.
        let mut success = 0usize;
        for (i, a) in traj.actions.iter().enumerate() {
            let good_action = matches!(a.name.as_str(), "stabilize" | "exploit" | "observe");
            let low_follow = traj
                .prediction_errors
                .get(i)
                .map(|e| *e < mean_error + 0.05)
                .unwrap_or(false);
            if good_action || low_follow {
                success += 1;
            }
        }
        let success_rate = if traj.actions.is_empty() {
            0.5
        } else {
            success as f64 / traj.actions.len() as f64
        };

        Ok(Abstraction {
            summary: format!(
                "obs={} mean_err={:.3} var={:.3} actions={} success={:.2}",
                traj.observations.len(),
                mean_error,
                error_variance,
                traj.actions.len(),
                success_rate
            ),
            mean_error,
            error_variance,
            action_count: traj.actions.len(),
            success_rate,
            action_dist,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Action, Observation, Trajectory};
    use chrono::Utc;
    use std::collections::HashMap;

    fn empty_traj() -> Trajectory {
        Trajectory {
            observations: vec![],
            predictions: vec![],
            actions: vec![],
            outcomes: vec![],
            prediction_errors: vec![],
            meta: HashMap::new(),
        }
    }

    #[test]
    fn distill_empty() {
        let abs = Abstractor.distill(&empty_traj()).unwrap();
        assert_eq!(abs.mean_error, 0.0);
        assert_eq!(abs.action_count, 0);
        assert!((abs.success_rate - 0.5).abs() < 1e-6);
    }

    #[test]
    fn distill_with_errors_and_actions() {
        let mut traj = empty_traj();
        traj.prediction_errors = vec![0.1, 0.5, 0.3];
        traj.actions = vec![
            Action {
                name: "stabilize".into(),
                params: HashMap::new(),
                constraints_checked: true,
            },
            Action {
                name: "explore".into(),
                params: HashMap::new(),
                constraints_checked: true,
            },
        ];
        traj.observations.push(Observation {
            timestamp: Utc::now(),
            modality: crate::types::Modality::Structured,
            content: serde_json::json!({}),
            confidence: 1.0,
            source: "test".into(),
        });
        let abs = Abstractor.distill(&traj).unwrap();
        assert!((abs.mean_error - 0.3).abs() < 1e-6);
        assert!(abs.error_variance > 0.0);
        assert_eq!(abs.action_count, 2);
        assert!(abs.action_dist.contains_key("stabilize"));
    }
}
