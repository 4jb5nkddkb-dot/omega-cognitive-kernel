#[allow(unused_imports)]
use crate::prelude::*;

pub mod abstractor;
pub mod learner;
pub mod patch;
pub mod reflector;
pub mod sandbox;
pub mod spectral;
pub mod store;
pub use spectral::{L1SpectralReflector, SpectralBin, SpectralPatch};

use crate::error::Result;
use crate::memory::MemoryFacade;
use crate::types::Trajectory;
use abstractor::Abstractor;
use hashbrown::HashMap;
use learner::Learner;
use reflector::{build_patch_context, Reflector};
use sandbox::CodeSandbox;
use store::VersionedStore;

#[derive(Debug)]
pub struct L1Result {
    pub patched: bool,
    pub patch_id: Option<uuid::Uuid>,
    pub reason: Option<String>,
    pub effect_damping: Option<f64>,
    /// Present only after an accepted effect is restored due to harmful feedback.
    /// Rejected patches are exposed separately as safety_degraded.
    pub rollback_effect: Option<crate::types::RuleEffect>,
    pub safety_degraded: bool,
}

pub struct L1CognitiveLayer {
    pub abstractor: Abstractor,
    pub reflector: Reflector,
    pub sandbox: CodeSandbox,
    pub store: VersionedStore,
    /// Available for post-patch learning hooks; primary update lives in L0.
    pub learner: Learner,
    /// Steps since the last accepted patch (for cooldown).
    pub steps_since_last_patch: u64,
    /// Master switch: when false, no cognitive code/rule patches are applied.
    /// Set from KernelConfig.self_modify_enabled (default false).
    pub self_modify_enabled: bool,
    pub previous_effect: Option<crate::types::RuleEffect>,
    pub pending_rollback: bool,
}

impl L1CognitiveLayer {
    pub fn new() -> Self {
        Self {
            abstractor: Abstractor,
            reflector: Reflector::default(),
            sandbox: CodeSandbox::default(),
            store: VersionedStore::new(),
            learner: Learner::default(),
            steps_since_last_patch: 100, // allow first patch immediately when enabled
            self_modify_enabled: false,
            previous_effect: None,
            pending_rollback: false,
        }
    }

    pub fn process(
        &mut self,
        trajectory: &Trajectory,
        memory: &mut MemoryFacade,
    ) -> Result<L1Result> {
        self.steps_since_last_patch = self.steps_since_last_patch.saturating_add(1);

        let abstraction = self.abstractor.distill(trajectory)?;

        // Hard gate: no self-modification unless explicitly enabled.
        if !self.self_modify_enabled {
            return Ok(L1Result {
                patched: false,
                patch_id: None,
                reason: Some("self_modify_enabled=false".into()),
                effect_damping: None,
                rollback_effect: None,
                safety_degraded: false,
            });
        }

        let mut strategy_weights = HashMap::new();
        for s in memory.strategy.retrieve("", 16) {
            strategy_weights.insert(s.name, s.weight);
        }
        let recent_errors: Vec<f64> = memory
            .case
            .retrieve("", 8)
            .iter()
            .map(|c| c.error_mean)
            .collect();
        let ctx = build_patch_context(
            memory.cas.avg_prediction_error,
            strategy_weights,
            &recent_errors,
        );

        if let Some(mut patch) =
            self.reflector
                .reflect(trajectory, &abstraction, &ctx, self.steps_since_last_patch)?
        {
            patch.effect = patch.effect.sanitize();
            let report = self.sandbox.validate(&patch)?;
            if report.accepted {
                let damping = patch.effect.error_damping;
                self.previous_effect = Some(memory.cognitive_effect.clone());
                self.pending_rollback = true;
                memory.cognitive_effect = patch.effect.clone();
                self.store.commit(patch, &report)?;
                self.steps_since_last_patch = 0;
                return Ok(L1Result {
                    patched: true,
                    patch_id: Some(report.patch_id),
                    reason: None,
                    effect_damping: Some(damping),
                    rollback_effect: None,
                    safety_degraded: false,
                });
            } else {
                return Ok(L1Result {
                    patched: false,
                    patch_id: None,
                    reason: Some(report.message),
                    effect_damping: None,
                    rollback_effect: None,
                    safety_degraded: true,
                });
            }
        }

        Ok(L1Result {
            patched: false,
            patch_id: None,
            reason: None,
            effect_damping: None,
            rollback_effect: None,
            safety_degraded: false,
        })
    }
    /// A worsening error after an accepted patch makes its previous RuleEffect
    /// eligible for a real rollback. Rejected patches never enter this path.
    pub fn observe_consequence(&mut self, error_delta: f64) -> bool {
        if self.pending_rollback && error_delta > 0.05 {
            return true;
        }
        self.pending_rollback = false;
        false
    }

    pub fn take_rollback_effect(&mut self) -> Option<crate::types::RuleEffect> {
        self.pending_rollback = false;
        self.previous_effect.take()
    }
}

impl Default for L1CognitiveLayer {
    fn default() -> Self {
        Self::new()
    }
}
