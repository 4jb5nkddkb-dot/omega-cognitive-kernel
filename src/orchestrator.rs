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
        let l2_result =
            self.l2
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
