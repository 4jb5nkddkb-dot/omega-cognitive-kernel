pub mod cas;
pub mod case;
pub mod strategy;

use crate::error::Result;
use crate::types::{RuleEffect, Trajectory};
use cas::CasState;
use case::CaseMemory;
use std::collections::HashMap;
use strategy::StrategyMemory;

#[derive(Debug, Default)]
pub struct MemoryFacade {
    pub case: CaseMemory,
    pub strategy: StrategyMemory,
    pub cas: CasState,
    /// Active structured cognitive effect (updated when L1 commits a patch).
    pub cognitive_effect: RuleEffect,
}

impl MemoryFacade {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn retrieve(&self, query: &str, k: usize) -> Result<HashMap<String, serde_json::Value>> {
        let mut ctx = HashMap::new();
        let cases = self.case.retrieve(query, k);
        let strategies = self.strategy.retrieve(query, k);
        ctx.insert(
            "cases".into(),
            serde_json::to_value(cases).unwrap_or_default(),
        );
        ctx.insert(
            "strategies".into(),
            serde_json::to_value(strategies).unwrap_or_default(),
        );
        ctx.insert(
            "cas".into(),
            serde_json::to_value(&self.cas).unwrap_or_default(),
        );
        ctx.insert(
            "cognitive_effect".into(),
            serde_json::to_value(&self.cognitive_effect).unwrap_or_default(),
        );
        Ok(ctx)
    }

    pub fn update(&mut self, traj: &Trajectory) -> Result<()> {
        self.case.add(traj);
        self.strategy.absorb(traj);
        self.cas.update(traj);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Action, Trajectory};
    use std::collections::HashMap;

    fn sample_traj() -> Trajectory {
        let mut meta = HashMap::new();
        meta.insert("goal".into(), serde_json::json!("test"));
        Trajectory {
            observations: vec![],
            predictions: vec![],
            actions: vec![Action {
                name: "observe".into(),
                params: HashMap::new(),
                constraints_checked: true,
            }],
            outcomes: vec![],
            prediction_errors: vec![0.15],
            meta,
        }
    }

    #[test]
    fn update_and_retrieve_roundtrip() {
        let mut mem = MemoryFacade::new();
        mem.update(&sample_traj()).unwrap();
        let ctx = mem.retrieve("test", 3).unwrap();
        assert!(ctx.contains_key("cases"));
        assert!(ctx.contains_key("strategies"));
        assert!(ctx.contains_key("cas"));
        assert_eq!(mem.cas.total_steps, 1);
    }
}
