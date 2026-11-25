#[allow(unused_imports)]
use crate::prelude::*;

use alloc::collections::BTreeMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IntrinsicSignal {
    pub competence: f64,
    pub novelty: f64,
    pub exploration: f64,
    pub combined: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntrinsicMotivation {
    pub baseline_state: BTreeMap<String, f64>,
    pub alpha: f64,
}

impl Default for IntrinsicMotivation {
    fn default() -> Self {
        Self {
            baseline_state: BTreeMap::new(),
            alpha: 0.15,
        }
    }
}

impl IntrinsicMotivation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn evaluate(
        &mut self,
        state: &BTreeMap<String, f64>,
        recent_progress: f64,
        success_rate: f64,
        stall: bool,
        mean_residual: f64,
        goal_distance: Option<f64>,
    ) -> IntrinsicSignal {
        let mut novelty_sum = 0.0;
        let mut count = 0usize;
        for (k, v) in state {
            let base = self.baseline_state.get(k).copied().unwrap_or(*v);
            novelty_sum += (v - base).abs();
            let updated = (1.0 - self.alpha) * base + self.alpha * (*v);
            self.baseline_state.insert(k.clone(), updated);
            count += 1;
        }
        let novelty = if count > 0 {
            (novelty_sum / count as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let prog_term = (recent_progress.max(0.0) * 2.0).clamp(0.0, 1.0);
        let competence = (0.6 * success_rate.clamp(0.0, 1.0) + 0.4 * prog_term).clamp(0.0, 1.0);

        let stall_boost = if stall { 0.45 } else { 0.0 };
        let residual_boost = (mean_residual.max(0.0) * 1.5).clamp(0.0, 0.4);
        let dist_boost = goal_distance.unwrap_or(0.0).clamp(0.0, 0.15);
        let exploration = (stall_boost + residual_boost + dist_boost).clamp(0.0, 1.0);

        let combined = (0.35 * competence + 0.25 * novelty + 0.40 * exploration).clamp(0.0, 1.0);
        IntrinsicSignal {
            competence,
            novelty,
            exploration,
            combined,
        }
    }

    pub fn modulate_progress(
        &self,
        expected_progress: f64,
        signal: &IntrinsicSignal,
        weight: f64,
    ) -> f64 {
        let w = weight.clamp(0.0, 1.0);
        expected_progress + w * signal.exploration * 0.15
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stall_boosts_exploration_and_stays_bounded() {
        let mut im = IntrinsicMotivation::new();
        let mut state = BTreeMap::new();
        state.insert("x".into(), 0.2);
        let sig = im.evaluate(&state, 0.0, 0.5, true, 0.2, Some(0.5));
        assert!(sig.exploration >= 0.45 && sig.exploration <= 1.0);
        assert!(sig.combined >= 0.0 && sig.combined <= 1.0);
        assert!(im.modulate_progress(0.1, &sig, 0.5) > 0.1);
    }
}
