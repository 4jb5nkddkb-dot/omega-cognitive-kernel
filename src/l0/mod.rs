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
        let actions = self.decision.decide(intent, &predictions, constraints)?;
        let outcomes = self.executor.execute(&actions)?;
        let errors = self.compute_errors(&predictions, &outcomes);

        let mut meta = HashMap::new();
        meta.insert("goal".into(), serde_json::Value::String(intent.goal.clone()));

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

    fn compute_errors(&self, preds: &[Prediction], _outcomes: &[serde_json::Value]) -> Vec<f64> {
        preds.iter().map(|p| p.uncertainty).collect()
    }
}
