pub mod abstractor;
pub mod learner;
pub mod patch;
pub mod reflector;
pub mod sandbox;
pub mod store;

use crate::error::Result;
use crate::memory::MemoryFacade;
use crate::types::Trajectory;
use abstractor::Abstractor;
use learner::Learner;
use reflector::{build_patch_context, Reflector};
use sandbox::CodeSandbox;
use std::collections::HashMap;
use store::VersionedStore;

#[derive(Debug)]
pub struct L1Result {
    pub patched: bool,
    pub patch_id: Option<uuid::Uuid>,
    pub reason: Option<String>,
    pub effect_damping: Option<f64>,
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
}

impl L1CognitiveLayer {
    pub fn new() -> Self {
        Self {
            abstractor: Abstractor,
            reflector: Reflector::default(),
            sandbox: CodeSandbox::default(),
            store: VersionedStore::new(),
            learner: Learner::default(),
            steps_since_last_patch: 100, // allow first patch immediately
        }
    }

    pub fn process(
        &mut self,
        trajectory: &Trajectory,
        memory: &mut MemoryFacade,
    ) -> Result<L1Result> {
        self.steps_since_last_patch = self.steps_since_last_patch.saturating_add(1);

        let abstraction = self.abstractor.distill(trajectory)?;

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
                memory.cognitive_effect = patch.effect.clone();
                self.store.commit(patch, &report)?;
                self.steps_since_last_patch = 0;
                return Ok(L1Result {
                    patched: true,
                    patch_id: Some(report.patch_id),
                    reason: None,
                    effect_damping: Some(damping),
                });
            } else {
                return Ok(L1Result {
                    patched: false,
                    patch_id: None,
                    reason: Some(report.message),
                    effect_damping: None,
                });
            }
        }

        Ok(L1Result {
            patched: false,
            patch_id: None,
            reason: None,
            effect_damping: None,
        })
    }
}

impl Default for L1CognitiveLayer {
    fn default() -> Self {
        Self::new()
    }
}
