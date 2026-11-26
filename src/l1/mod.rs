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
use crate::types::{CognitivePatch, Trajectory};
use abstractor::{Abstraction, Abstractor};
use hashbrown::HashMap;
use learner::Learner;
use patch::PatchContext;
use reflector::{build_patch_context, Reflector};
use sandbox::{CodeSandbox, ValidationReport};
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
    pub shadow_mode: bool,
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
            shadow_mode: false,
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
        if !self.self_modify_enabled && !self.shadow_mode {
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
                if self.shadow_mode {
                    self.steps_since_last_patch = 0;
                    return Ok(L1Result {
                        patched: false,
                        patch_id: Some(patch.patch_id),
                        reason: Some("shadow_mode: validated without applying".into()),
                        effect_damping: Some(patch.effect.error_damping),
                        rollback_effect: None,
                        safety_degraded: false,
                    });
                }
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
    /// Generate and statically validate a patch without changing store, memory,
    /// cooldown, rollback state, or any durable sidecar. This is not a reward
    /// prediction: external efficacy requires a paired environment evaluation.
    pub fn dry_run_patch(
        &self,
        trajectory: &Trajectory,
        abstraction: &Abstraction,
        ctx: &PatchContext,
    ) -> Result<Option<(CognitivePatch, ValidationReport)>> {
        let Some(patch) =
            self.reflector
                .reflect(trajectory, abstraction, ctx, self.steps_since_last_patch)?
        else {
            return Ok(None);
        };
        let report = self.sandbox.validate(&patch)?;
        Ok(Some((patch, report)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_patch_does_not_commit_or_advance_cooldown() {
        let layer = L1CognitiveLayer::new();
        let trajectory = Trajectory {
            observations: vec![],
            predictions: vec![],
            actions: vec![],
            outcomes: vec![],
            prediction_errors: vec![0.8],
            meta: HashMap::new(),
        };
        let abstraction = Abstraction {
            summary: "test".into(),
            mean_error: 0.8,
            error_variance: 0.2,
            action_count: 0,
            success_rate: 0.1,
            action_dist: HashMap::new(),
        };
        let (_, report) = layer
            .dry_run_patch(&trajectory, &abstraction, &PatchContext::default())
            .unwrap()
            .expect("safe candidate");
        assert!(report.accepted);
        assert!(layer.store.history.is_empty());
        assert_eq!(layer.steps_since_last_patch, 100);
        assert!(!layer.pending_rollback);
    }
    #[test]
    fn shadow_mode_validates_without_mutating_effect() {
        let mut l1 = L1CognitiveLayer::new();
        l1.shadow_mode = true;
        l1.reflector.error_threshold = 0.1;
        let trajectory = Trajectory {
            observations: vec![],
            predictions: vec![],
            actions: vec![],
            outcomes: vec![],
            prediction_errors: vec![0.8],
            meta: HashMap::new(),
            };
        let mut memory = MemoryFacade::default();
        let res = l1
            .process(&trajectory, &mut memory)
            .expect("shadow process");
        assert!(!res.patched);
        assert!(res.patch_id.is_some());
        assert!(memory.cognitive_effect == crate::types::RuleEffect::default());
        assert!(!l1.pending_rollback);
    }
    }
