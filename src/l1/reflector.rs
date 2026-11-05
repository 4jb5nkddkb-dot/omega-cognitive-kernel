use crate::error::Result;
use crate::l1::abstractor::Abstraction;
use crate::l1::patch::{PatchContext, PatchGenerator};
use crate::types::{CognitivePatch, Trajectory};
use std::collections::HashMap;

pub struct Reflector {
    pub error_threshold: f64,
    /// Minimum steps between accepted patches (cooldown).
    pub min_steps_between_patches: u64,
}

impl Default for Reflector {
    fn default() -> Self {
        Self {
            error_threshold: 0.30,
            min_steps_between_patches: 3,
        }
    }
}

impl Reflector {
    pub fn reflect(
        &self,
        traj: &Trajectory,
        abs: &Abstraction,
        ctx: &PatchContext,
        steps_since_last_patch: u64,
    ) -> Result<Option<CognitivePatch>> {
        // Cooldown gate
        if steps_since_last_patch < self.min_steps_between_patches {
            return Ok(None);
        }

        // Skip if error is low, we have actions, variance is not exploding, and streak is short.
        let stable = abs.mean_error < self.error_threshold
            && abs.action_count > 0
            && abs.error_variance < 0.08
            && abs.success_rate > 0.45
            && ctx.high_error_streak < 2;

        if stable {
            return Ok(None);
        }

        let patch = PatchGenerator::generate_with_context(traj, &abs.summary, ctx);
        Ok(Some(patch))
    }

    /// Convenience when no rich context / cooldown is available.
    pub fn reflect_simple(
        &self,
        traj: &Trajectory,
        abs: &Abstraction,
    ) -> Result<Option<CognitivePatch>> {
        self.reflect(traj, abs, &PatchContext::default(), 100)
    }
}

/// Build a PatchContext from memory-style maps (used by L1).
pub fn build_patch_context(
    recent_mean_error: f64,
    strategy_weights: HashMap<String, f64>,
    recent_errors: &[f64],
) -> PatchContext {
    let high_error_streak = recent_errors
        .iter()
        .rev()
        .take(5)
        .filter(|&&e| e > 0.42)
        .count() as u32;
    PatchContext {
        recent_mean_error,
        strategy_weights,
        high_error_streak,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l1::abstractor::Abstraction;
    use crate::types::Trajectory;
    use std::collections::HashMap;

    fn abs(mean: f64, var: f64, success: f64) -> Abstraction {
        Abstraction {
            summary: "test".into(),
            mean_error: mean,
            error_variance: var,
            action_count: 3,
            success_rate: success,
            action_dist: HashMap::new(),
        }
    }

    fn empty_traj() -> Trajectory {
        Trajectory {
            observations: vec![],
            predictions: vec![],
            actions: vec![],
            outcomes: vec![],
            prediction_errors: vec![0.5],
            meta: HashMap::new(),
        }
    }

    #[test]
    fn cooldown_blocks_patch() {
        let r = Reflector::default();
        let a = abs(0.9, 0.2, 0.1);
        let out = r
            .reflect(&empty_traj(), &a, &PatchContext::default(), 1)
            .unwrap();
        assert!(out.is_none());
    }

    #[test]
    fn high_error_triggers_after_cooldown() {
        let r = Reflector::default();
        let a = abs(0.9, 0.2, 0.1);
        let out = r
            .reflect(&empty_traj(), &a, &PatchContext::default(), 10)
            .unwrap();
        assert!(out.is_some());
    }

    #[test]
    fn stable_low_error_skips() {
        let r = Reflector::default();
        let a = abs(0.15, 0.01, 0.8);
        let out = r
            .reflect(&empty_traj(), &a, &PatchContext::default(), 10)
            .unwrap();
        assert!(out.is_none());
    }
}
