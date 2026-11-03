use crate::types::Trajectory;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Case {
    pub summary: String,
    pub goal: String,
    pub error_mean: f64,
    pub timestamp: chrono::DateTime<Utc>,
}

#[derive(Debug)]
pub struct CaseMemory {
    cases: VecDeque<Case>,
    capacity: usize,
}

impl CaseMemory {
    pub fn new(capacity: usize) -> Self {
        Self {
            cases: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn add(&mut self, traj: &Trajectory) {
        let error_mean = if traj.prediction_errors.is_empty() {
            0.0
        } else {
            traj.prediction_errors.iter().sum::<f64>() / traj.prediction_errors.len() as f64
        };

        let summary = format!(
            "actions={} errors_mean={:.3}",
            traj.actions.len(),
            error_mean
        );

        let case = Case {
            summary,
            goal: traj
                .meta
                .get("goal")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            error_mean,
            timestamp: Utc::now(),
        };

        if self.cases.len() >= self.capacity {
            self.cases.pop_front();
        }
        self.cases.push_back(case);
    }

    pub fn retrieve(&self, _query: &str, k: usize) -> Vec<Case> {
        self.cases.iter().rev().take(k).cloned().collect()
    }
}

impl Default for CaseMemory {
    fn default() -> Self {
        Self::new(256)
    }
}
