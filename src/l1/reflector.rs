use crate::error::Result;
use crate::l1::abstractor::Abstraction;
use crate::l1::patch::{PatchContext, PatchGenerator};
use crate::types::{CognitivePatch, Trajectory};
use std::collections::HashMap;

pub struct Reflector {
    pub error_threshold: f64,
}

impl Default for Reflector {
    fn default() -> Self {
        Self {
            error_threshold: 0.30,
        }
    }
}

impl Reflector {
    pub fn reflect(
        &self,
        traj: &Trajectory,
        abs: &Abstraction,
        ctx: &PatchContext,
    ) -> Result<Option<CognitivePatch>> {
        if abs.mean_error < self.error_threshold && abs.action_count > 0 && ctx.high_error_streak < 2
        {
            return Ok(None);
        }
        let patch = PatchGenerator::generate_with_context(traj, &abs.summary, ctx);
        Ok(Some(patch))
    }

    /// Convenience when no rich context is available.
    pub fn reflect_simple(
        &self,
        traj: &Trajectory,
        abs: &Abstraction,
    ) -> Result<Option<CognitivePatch>> {
        self.reflect(traj, abs, &PatchContext::default())
    }
}

/// Build a PatchContext from memory-style maps (used by L1).
pub fn build_patch_context(
    recent_mean_error: f64,
    strategy_weights: HashMap<String, f64>,
    recent_errors: &[f64],
) -> PatchContext {
    let high_error_streak = recent_errors.iter().rev().take(5).filter(|&&e| e > 0.42).count() as u32;
    PatchContext {
        recent_mean_error,
        strategy_weights,
        high_error_streak,
    }
}
