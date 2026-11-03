pub mod observer;
pub mod predictor;
pub mod decision;
pub mod executor;

use crate::error::Result;
use crate::memory::MemoryFacade;
use crate::types::*;
use std::collections::HashMap;

pub struct L0WorldLayer<O, P, D, E>
where
    O: observer::Observer,
    P: predictor::Predictor,
    D: decision::DecisionMaker,
    E: executor::Executor,
{
    pub observer: O,
    pub predictor: P,
    pub decision: D,
    pub executor: E,
    pub memory: MemoryFacade,
}

impl<O, P, D, E> L0WorldLayer<O, P, D, E>
where
    O: observer::Observer,
    P: predictor::Predictor,
    D: decision::DecisionMaker,
    E: executor::Executor,
{
    pub fn step(
        &mut self,
        intent: &Intent,
        constraints: &HashMap<String, f64>,
    ) -> Result<Trajectory> {
        let observations = self.observer.observe()?;
        let mem_ctx = self.memory.retrieve(&intent.goal, 5)?;
        let predictions = self.predictor.predict(&observations, &mem_ctx)?;
        let actions = self
            .decision
            .decide(intent, &predictions, constraints, &mem_ctx)?;
        let outcomes = self.executor.execute(&actions)?;
        let errors = self.compute_errors(&predictions, &actions);

        let mut meta = HashMap::new();
        meta.insert("goal".into(), serde_json::Value::String(intent.goal.clone()));
        meta.insert(
            "damping".into(),
            serde_json::json!(self.memory.cognitive_effect.error_damping),
        );
        if let Some(a) = actions.first() {
            meta.insert("chosen_action".into(), serde_json::json!(a.name));
        }

        let traj = Trajectory {
            observations,
            predictions,
            actions,
            outcomes,
            prediction_errors: errors,
            meta,
        };

        self.memory.update(&traj)?;
        Ok(traj)
    }

    /// Error tracks predictor uncertainty, with a small action-quality term.
    fn compute_errors(&self, preds: &[Prediction], actions: &[Action]) -> Vec<f64> {
        let action_penalty = match actions.first().map(|a| a.name.as_str()) {
            Some("stabilize") => -0.05,
            Some("exploit") => -0.02,
            Some("explore") => 0.03,
            _ => 0.0,
        };
        preds
            .iter()
            .map(|p| (p.uncertainty + action_penalty).clamp(0.05, 1.0))
            .collect()
    }
}
