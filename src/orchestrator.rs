use crate::error::Result;
use crate::l0::L0WorldLayer;
use crate::l1::{L1CognitiveLayer, L1Result};
use crate::l2::{L2MetaLayer, L2Result};
use crate::types::{Intent, SystemState, Trajectory};
use std::collections::HashMap;

pub struct OmegaOrchestrator<O, P, D, E>
where
    O: crate::l0::observer::Observer,
    P: crate::l0::predictor::Predictor,
    D: crate::l0::decision::DecisionMaker,
    E: crate::l0::executor::Executor,
{
    pub l0: L0WorldLayer<O, P, D, E>,
    pub l1: L1CognitiveLayer,
    pub l2: L2MetaLayer,
    pub biases: HashMap<String, f64>,
    pub constraints: HashMap<String, f64>,
    pub total_steps: u64,
}

#[derive(Debug)]
pub struct StepResult {
    pub trajectory: Trajectory,
    pub l1: L1Result,
    pub l2: L2Result,
}

impl<O, P, D, E> OmegaOrchestrator<O, P, D, E>
where
    O: crate::l0::observer::Observer,
    P: crate::l0::predictor::Predictor,
    D: crate::l0::decision::DecisionMaker,
    E: crate::l0::executor::Executor,
{
    pub fn run_once(&mut self, high_level_goal: &str) -> Result<StepResult> {
        self.total_steps += 1;

        let intent = Intent {
            goal: high_level_goal.to_string(),
            priority: 1.0,
            bias_from_memory: self.biases.clone(),
        };

        // L0
        let trajectory = self.l0.step(&intent, &self.constraints)?;

        // L1
        let l1_result = self.l1.process(&trajectory, &mut self.l0.memory)?;

        // Build system state for L2
        let avg_err = self.l0.memory.cas.avg_prediction_error;
        let state = SystemState {
            active_code: self.l1.store.active.clone(),
            highest_attractor: high_level_goal.to_string(),
            generative_base: self
                .l1
                .store
                .active
                .get("cognitive_rules")
                .cloned()
                .or(Some("dayan_generative_base".into())),
            last_trajectory_summary: Some(format!(
                "step={} actions={}",
                self.total_steps,
                trajectory.actions.len()
            )),
            avg_prediction_error: avg_err,
            total_steps: self.total_steps,
        };

        // L2
        let l2_result = self
            .l2
            .step(&state, self.biases.clone(), self.constraints.clone())?;
        self.biases = l2_result.biases.clone();
        self.constraints = l2_result.constraints.clone();

        Ok(StepResult {
            trajectory,
            l1: l1_result,
            l2: l2_result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l0::decision::ConstrainedDecisionMaker;
    use crate::l0::executor::SandboxedExecutor;
    use crate::l0::observer::MultiModalObserver;
    use crate::l0::predictor::ModelBasedPredictor;
    use crate::l0::L0WorldLayer;
    use crate::l1::L1CognitiveLayer;
    use crate::l2::L2MetaLayer;
    use crate::memory::MemoryFacade;
    use crate::safety;

    fn make_kernel() -> OmegaOrchestrator<
        MultiModalObserver,
        ModelBasedPredictor,
        ConstrainedDecisionMaker,
        SandboxedExecutor,
    > {
        let l0 = L0WorldLayer {
            observer: MultiModalObserver::new(),
            predictor: ModelBasedPredictor,
            decision: ConstrainedDecisionMaker,
            executor: SandboxedExecutor::new(vec![
                "noop".into(),
                "observe".into(),
                "explore".into(),
                "exploit".into(),
                "stabilize".into(),
            ]),
            memory: MemoryFacade::new(),
        };
        OmegaOrchestrator {
            l0,
            l1: L1CognitiveLayer::new(),
            l2: L2MetaLayer::new(),
            biases: HashMap::new(),
            constraints: safety::default_constraints(),
            total_steps: 0,
        }
    }

    #[test]
    fn run_once_increments_steps() {
        let mut kernel = make_kernel();
        let r = kernel
            .run_once("maintain coherent self-model")
            .expect("run_once should succeed");
        assert_eq!(kernel.total_steps, 1);
        assert!(!r.trajectory.observations.is_empty());
    }

    #[test]
    fn multiple_steps_accumulate_memory() {
        let mut kernel = make_kernel();
        for _ in 0..3 {
            kernel
                .run_once("maintain coherent self-model")
                .expect("step ok");
        }
        assert_eq!(kernel.total_steps, 3);
        assert!(kernel.l0.memory.cas.total_steps >= 3);
    }

    /// S1 gate: high error triggers a patch; subsequent steps show lower avg error
    /// and active damping < 1.0 (structured effect changed behavior).
    #[test]
    fn s1_patch_reduces_prediction_error() {
        let mut kernel = make_kernel();
        // Force reflector to be sensitive.
        kernel.l1.reflector.error_threshold = 0.25;

        let mut early_err = 0.0;
        let mut saw_patch = false;

        for step in 0..8 {
            let r = kernel
                .run_once("reduce prediction error under uncertainty")
                .expect("step");
            if step == 0 {
                early_err = kernel.l0.memory.cas.avg_prediction_error;
            }
            if r.l1.patched {
                saw_patch = true;
                assert!(
                    r.l1.effect_damping.unwrap_or(1.0) < 1.0,
                    "committed patch should lower damping"
                );
            }
        }

        assert!(
            saw_patch,
            "expected at least one accepted patch under high error"
        );
        assert!(
            kernel.l0.memory.cognitive_effect.error_damping < 1.0,
            "active cognitive effect must damp errors"
        );
        let final_err = kernel.l0.memory.cas.avg_prediction_error;
        assert!(
            final_err < early_err,
            "avg error should decrease after rule commit: early={early_err:.3} final={final_err:.3}"
        );
    }

    #[test]
    fn s1_decision_uses_rule_preference() {
        let mut kernel = make_kernel();
        // Seed a strong stabilize preference as if a patch was committed.
        kernel
            .l0
            .memory
            .cognitive_effect
            .action_preference
            .insert("stabilize".into(), 3.0);
        kernel.l0.memory.cognitive_effect.error_damping = 0.6;

        let r = kernel.run_once("test preference").expect("step");
        let name = &r.trajectory.actions[0].name;
        assert_eq!(
            name, "stabilize",
            "decision should prefer stabilize from RuleEffect"
        );
    }
}
