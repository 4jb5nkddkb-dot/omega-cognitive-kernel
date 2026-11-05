use crate::error::Result;
use crate::memory::MemoryFacade;
use crate::types::Trajectory;

/// Owns post-trajectory learning: case/strategy/CAS update + idle strategy decay.
pub struct Learner {
    pub decay_idle_threshold: u32,
    pub decay_factor: f64,
}

impl Default for Learner {
    fn default() -> Self {
        Self {
            decay_idle_threshold: 5,
            decay_factor: 0.92,
        }
    }
}

impl Learner {
    pub fn update(&self, memory: &mut MemoryFacade, traj: &Trajectory) -> Result<()> {
        memory.update(traj)?;
        memory
            .strategy
            .decay_idle(self.decay_idle_threshold, self.decay_factor);
        Ok(())
    }

    /// Decay-only pass (when L0 already called memory.update).
    pub fn decay_only(&self, memory: &mut MemoryFacade) {
        memory
            .strategy
            .decay_idle(self.decay_idle_threshold, self.decay_factor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Action, Trajectory};
    use std::collections::HashMap;

    fn traj(name: &str) -> Trajectory {
        Trajectory {
            observations: vec![],
            predictions: vec![],
            actions: vec![Action {
                name: name.into(),
                params: HashMap::new(),
                constraints_checked: true,
            }],
            outcomes: vec![],
            prediction_errors: vec![0.2],
            meta: HashMap::new(),
        }
    }

    #[test]
    fn learner_updates_and_decays() {
        let learner = Learner::default();
        let mut mem = MemoryFacade::new();
        learner.update(&mut mem, &traj("explore")).unwrap();
        for _ in 0..8 {
            learner.update(&mut mem, &traj("observe")).unwrap();
        }
        let explore = mem
            .strategy
            .retrieve("", 10)
            .into_iter()
            .find(|s| s.name == "explore");
        assert!(explore.is_some());
        assert!(explore.unwrap().weight <= 1.05);
    }
}
