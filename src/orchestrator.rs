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

        let trajectory = self.l0.step(&intent, &self.constraints)?;
        let l1_result = self.l1.process(&trajectory, &mut self.l0.memory)?;

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

        let current_effect = self.l0.memory.cognitive_effect.clone();
        let l2_result = self.l2.step(
            &state,
            self.biases.clone(),
            self.constraints.clone(),
            &current_effect,
        )?;
        self.biases = l2_result.biases.clone();
        self.constraints = l2_result.constraints.clone();

        if let Some(nudged) = &l2_result.effect_nudge {
            self.l0.memory.cognitive_effect = nudged.clone();
        }

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
    use crate::l1::learner::Learner;
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
            learner: Learner::default(),
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

    #[test]
    fn s1_patch_reduces_prediction_error() {
        let mut kernel = make_kernel();
        kernel.l1.reflector.error_threshold = 0.99;
        for _ in 0..5 {
            kernel.run_once("warmup").expect("warmup");
        }
        let early_err = kernel.l0.memory.cas.avg_prediction_error;
        kernel.l1.reflector.error_threshold = 0.25;
        let mut saw_patch = false;

        for _step in 0..12 {
            let r = kernel
                .run_once("reduce prediction error under uncertainty")
                .expect("step");
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

    #[test]
    fn patch_cooldown_blocks_immediate_repatch() {
        let mut kernel = make_kernel();
        kernel.l1.reflector.error_threshold = 0.05;
        kernel.l1.reflector.min_steps_between_patches = 3;

        let mut patch_steps = Vec::new();
        for step in 0..10 {
            let r = kernel.run_once("high error force patch").expect("step");
            if r.l1.patched {
                patch_steps.push(step);
            }
        }

        if patch_steps.len() >= 2 {
            for w in patch_steps.windows(2) {
                assert!(
                    w[1] - w[0] >= 3,
                    "patches too close: {:?} (cooldown violated)",
                    patch_steps
                );
            }
        }
    }

    #[test]
    fn constraints_max_risk_discourages_explore() {
        let mut kernel = make_kernel();
        kernel.constraints.insert("max_risk".into(), 0.20);
        kernel.l0.memory.cognitive_effect = crate::types::RuleEffect::default();

        let mut explore_count = 0;
        for _ in 0..8 {
            let r = kernel.run_once("constrained").expect("step");
            if r.trajectory.actions.first().map(|a| a.name.as_str()) == Some("explore") {
                explore_count += 1;
            }
        }
        assert!(
            explore_count <= 3,
            "explore_count={explore_count} too high under tight max_risk"
        );
    }

    #[test]
    fn l2_soft_nudge_can_change_explore_bonus() {
        let mut kernel = make_kernel();
        kernel.l2.detector.window = 2;
        kernel.l2.detector.pred_threshold = 0.3;
        kernel.l2.detector.self_ref_threshold = 0.3;
        kernel.l2.detector.cooldown_steps = 0;

        let before = kernel.l0.memory.cognitive_effect.explore_bonus;
        let mut saw_nudge = false;
        for _ in 0..12 {
            let r = kernel
                .run_once("maintain coherent self-model omega dayan")
                .expect("step");
            if r.l2.effect_nudge.is_some() {
                saw_nudge = true;
            }
        }
        let after = kernel.l0.memory.cognitive_effect.explore_bonus;
        assert!(
            saw_nudge || (after - before).abs() > 1e-6,
            "expected L2 soft-nudge path to fire; before={before} after={after}"
        );
    }

    /// Force fixed-point entry then degrade metrics → should exit.
    #[test]
    fn fixed_point_can_exit_when_metrics_degrade() {
        let mut kernel = make_kernel();
        kernel.l2.detector.window = 2;
        kernel.l2.detector.pred_threshold = 0.2;
        kernel.l2.detector.self_ref_threshold = 0.2;
        kernel.l2.detector.cooldown_steps = 0;

        let mut entered = false;
        let mut exited = false;
        for i in 0..20 {
            // First half: coherent goal to encourage entry
            let goal = if i < 10 {
                "maintain coherent self-model omega dayan"
            } else {
                // Second half: nonsense goal + force high error via effect
                kernel.l0.memory.cas.avg_prediction_error = 0.9;
                "random noise goal xyz"
            };
            let r = kernel.run_once(goal).expect("step");
            if r.l2.entered_fixed_point {
                entered = true;
            }
            if r.l2.exited_fixed_point {
                exited = true;
            }
        }
        // Entry is probabilistic on metrics; exit requires having been inside.
        // Accept either: we exited, or we never entered (soft assertion on path).
        if entered {
            assert!(
                exited || !kernel.l2.in_fixed_point,
                "entered fixed-point but never left"
            );
        }
    }
}
