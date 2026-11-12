//! Case memory with **prioritized experience replay** (PER-style).
//!
//! - Priority ∝ `|error_mean|^α` (prediction error as TD-error proxy)
//! - Full buffer: **evict lowest priority** (not FIFO)
//! - `retrieve`: sample without replacement ∝ priority^α
//! - Optional IS weights for bias correction when training-style consumers need them

use crate::types::Trajectory;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Floor so zero-error cases still get tiny sample chance.
const PRIORITY_EPS: f64 = 1e-3;
/// Exponent on |error| when computing priority (classic PER uses α ∈ [0, 1]).
const PRIORITY_ALPHA: f64 = 0.6;
/// After a case is sampled, multiply priority by this (soften re-sampling dominance).
const PRIORITY_DECAY_ON_SAMPLE: f64 = 0.92;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Case {
    pub summary: String,
    pub goal: String,
    pub error_mean: f64,
    pub timestamp: chrono::DateTime<Utc>,
    /// Replay priority; higher = more likely to keep & sample.
    #[serde(default = "default_priority")]
    pub priority: f64,
}

fn default_priority() -> f64 {
    PRIORITY_EPS
}

impl Case {
    pub fn priority_from_error(error_mean: f64) -> f64 {
        let e = error_mean.abs().max(PRIORITY_EPS);
        e.powf(PRIORITY_ALPHA)
    }
}

#[derive(Debug)]
pub struct CaseMemory {
    cases: VecDeque<Case>,
    capacity: usize,
    /// Running sum of priorities for O(1) normalization hints (recomputed on structural change).
    priority_sum: f64,
}

impl CaseMemory {
    pub fn new(capacity: usize) -> Self {
        Self {
            cases: VecDeque::with_capacity(capacity),
            capacity,
            priority_sum: 0.0,
        }
    }

    pub fn len(&self) -> usize {
        self.cases.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cases.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    fn recompute_sum(&mut self) {
        self.priority_sum = self
            .cases
            .iter()
            .map(|c| c.priority.max(PRIORITY_EPS))
            .sum();
    }

    /// Index of minimum-priority case (oldest among ties).
    fn min_priority_index(&self) -> Option<usize> {
        if self.cases.is_empty() {
            return None;
        }
        let mut best_i = 0usize;
        let mut best_p = self.cases[0].priority;
        for (i, c) in self.cases.iter().enumerate().skip(1) {
            if c.priority < best_p - 1e-15 {
                best_i = i;
                best_p = c.priority;
            }
        }
        Some(best_i)
    }

    pub fn add(&mut self, traj: &Trajectory) {
        let error_mean = if traj.prediction_errors.is_empty() {
            0.0
        } else {
            traj.prediction_errors.iter().sum::<f64>() / traj.prediction_errors.len() as f64
        };

        let priority = Case::priority_from_error(error_mean);
        let summary = format!(
            "actions={} errors_mean={:.3} p={:.4}",
            traj.actions.len(),
            error_mean,
            priority
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
            priority,
        };

        if self.cases.len() >= self.capacity {
            // Evict lowest priority (PER buffer management), not FIFO.
            if let Some(i) = self.min_priority_index() {
                if let Some(old) = self.cases.remove(i) {
                    self.priority_sum -= old.priority.max(PRIORITY_EPS);
                }
            }
        }

        self.priority_sum += case.priority.max(PRIORITY_EPS);
        self.cases.push_back(case);
        if self.priority_sum < 0.0 || !self.priority_sum.is_finite() {
            self.recompute_sum();
        }
    }

    /// Deterministic recent-first listing (debug / UI).
    pub fn recent(&self, k: usize) -> Vec<Case> {
        self.cases.iter().rev().take(k).cloned().collect()
    }

    /// Prioritized sample without replacement. `query` reserved for future goal filtering.
    pub fn retrieve(&self, _query: &str, k: usize) -> Vec<Case> {
        if self.cases.is_empty() || k == 0 {
            return Vec::new();
        }
        let n = self.cases.len().min(k);
        let mut remaining: Vec<(usize, f64)> = self
            .cases
            .iter()
            .enumerate()
            .map(|(i, c)| (i, c.priority.max(PRIORITY_EPS)))
            .collect();

        let mut out = Vec::with_capacity(n);
        // Lightweight LCG for reproducible-enough sampling without new deps.
        let mut seed = (self.cases.len() as u64)
            .wrapping_mul(0x9E37_79B9)
            .wrapping_add(
                self.cases
                    .back()
                    .map(|c| c.error_mean.to_bits() ^ c.priority.to_bits())
                    .unwrap_or(1),
            );

        for _ in 0..n {
            let sum: f64 = remaining.iter().map(|(_, p)| *p).sum();
            if sum <= 0.0 || remaining.is_empty() {
                break;
            }
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let r = (seed >> 33) as f64 / (u32::MAX as f64) * sum;
            let mut acc = 0.0;
            let mut pick = 0usize;
            for (j, (_, p)) in remaining.iter().enumerate() {
                acc += *p;
                if r <= acc {
                    pick = j;
                    break;
                }
                pick = j;
            }
            let (idx, _) = remaining.remove(pick);
            out.push(self.cases[idx].clone());
        }
        out
    }

    /// Same as `retrieve`, but decays priorities of sampled cases (in-place) and
    /// returns approximate importance-sampling weights `w_i ∝ 1 / (N * P(i))`.
    pub fn retrieve_with_is_weights(&mut self, query: &str, k: usize) -> (Vec<Case>, Vec<f64>) {
        let sampled = self.retrieve(query, k);
        if sampled.is_empty() {
            return (sampled, Vec::new());
        }
        let n = self.cases.len().max(1) as f64;
        let sum = self.priority_sum.max(PRIORITY_EPS);
        let mut weights = Vec::with_capacity(sampled.len());

        for s in &sampled {
            // Match by timestamp + error (stable enough for in-memory buffer).
            if let Some(slot) = self
                .cases
                .iter_mut()
                .find(|c| c.timestamp == s.timestamp && (c.error_mean - s.error_mean).abs() < 1e-12)
            {
                let p = slot.priority.max(PRIORITY_EPS);
                let prob = p / sum;
                let w = 1.0 / (n * prob.max(1e-12));
                weights.push(w);
                self.priority_sum -= slot.priority.max(PRIORITY_EPS);
                slot.priority = (slot.priority * PRIORITY_DECAY_ON_SAMPLE).max(PRIORITY_EPS);
                self.priority_sum += slot.priority;
            } else {
                weights.push(1.0);
            }
        }

        // Normalize IS weights so max is 1 (common practical trick).
        if let Some(max_w) = weights.iter().cloned().fold(None, |a: Option<f64>, b| {
            Some(a.map(|x| x.max(b)).unwrap_or(b))
        }) {
            if max_w > 0.0 {
                for w in &mut weights {
                    *w /= max_w;
                }
            }
        }
        if !self.priority_sum.is_finite() {
            self.recompute_sum();
        }
        (sampled, weights)
    }

    pub fn snapshot(&self) -> (Vec<Case>, usize) {
        (self.cases.iter().cloned().collect(), self.capacity)
    }

    pub fn restore(cases: Vec<Case>, capacity: usize) -> Self {
        let mut mem = Self::new(capacity);
        for mut c in cases {
            if c.priority <= 0.0 || !c.priority.is_finite() {
                c.priority = Case::priority_from_error(c.error_mean);
            }
            if mem.cases.len() >= capacity {
                if let Some(i) = mem.min_priority_index() {
                    mem.cases.remove(i);
                }
            }
            mem.cases.push_back(c);
        }
        mem.recompute_sum();
        mem
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
    }

    #[test]
    fn evicts_lowest_priority_not_fifo() {
        let mut mem = CaseMemory::new(2);
        mem.add(&sample_traj("low", 0.05)); // low priority
        mem.add(&sample_traj("high", 0.9)); // high priority
        mem.add(&sample_traj("mid", 0.4)); // should kick out "low"
        let goals: Vec<_> = mem.cases.iter().map(|c| c.goal.as_str()).collect();
        assert!(
            !goals.contains(&"low"),
            "low-error case should be evicted: {goals:?}"
        );
        assert!(goals.contains(&"high"));
        assert!(goals.contains(&"mid"));
    }

    #[test]
    fn retrieve_prefers_high_error() {
        let mut mem = CaseMemory::new(20);
        for _ in 0..10 {
            mem.add(&sample_traj("easy", 0.02));
        }
        mem.add(&sample_traj("hard", 0.95));
        let mut hard_hits = 0;
        for _ in 0..40 {
            let batch = mem.retrieve("", 3);
            if batch.iter().any(|c| c.goal == "hard") {
                hard_hits += 1;
            }
        }
        // High-error case should appear often under priority sampling.
        assert!(hard_hits >= 8, "hard_hits={hard_hits}");
    }

    #[test]
    fn is_weights_normalized() {
        let mut mem = CaseMemory::new(10);
        mem.add(&sample_traj("a", 0.2));
        mem.add(&sample_traj("b", 0.8));
        let (cases, ws) = mem.retrieve_with_is_weights("", 2);
        assert_eq!(cases.len(), ws.len());
        assert!(ws.iter().all(|w| *w > 0.0 && *w <= 1.0 + 1e-9));
    }

    #[test]
    fn restore_backfills_priority() {
        let c = Case {
            summary: "x".into(),
            goal: "g".into(),
            error_mean: 0.5,
            timestamp: Utc::now(),
            priority: 0.0, // legacy
        };
        let mem = CaseMemory::restore(vec![c], 8);
        assert!(mem.cases[0].priority > 0.0);
    }
}
