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

    pub fn snapshot(&self) -> (Vec<Case>, usize) {
        (self.cases.iter().cloned().collect(), self.capacity)
    }

    pub fn restore(cases: Vec<Case>, capacity: usize) -> Self {
        let mut dq = VecDeque::with_capacity(capacity);
        for c in cases {
            if dq.len() >= capacity {
                dq.pop_front();
            }
            dq.push_back(c);
        }
        Self {
            cases: dq,
            capacity,
        }
    }
}

impl Default for CaseMemory {
    fn default() -> Self {
        Self::new(256)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Trajectory;
    use std::collections::HashMap;

    fn sample_traj(goal: &str, err: f64) -> Trajectory {
        let mut meta = HashMap::new();
        meta.insert("goal".into(), serde_json::Value::String(goal.into()));
        Trajectory {
            observations: vec![],
            predictions: vec![],
            actions: vec![],
            outcomes: vec![],
            prediction_errors: vec![err],
            meta,
        }
    }

    #[test]
    fn adds_and_retrieves_cases() {
        let mut mem = CaseMemory::new(10);
        mem.add(&sample_traj("explore", 0.3));
        mem.add(&sample_traj("exploit", 0.1));
        let cases = mem.retrieve("any", 5);
        assert_eq!(cases.len(), 2);
        assert_eq!(cases[0].goal, "exploit");
    }

    #[test]
    fn respects_capacity() {
        let mut mem = CaseMemory::new(2);
        mem.add(&sample_traj("a", 0.1));
        mem.add(&sample_traj("b", 0.2));
        mem.add(&sample_traj("c", 0.3));
        let cases = mem.retrieve("any", 10);
        assert_eq!(cases.len(), 2);
        assert_eq!(cases[0].goal, "c");
        assert_eq!(cases[1].goal, "b");
    }
}
