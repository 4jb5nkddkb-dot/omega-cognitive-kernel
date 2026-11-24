#[allow(unused_imports)]
use crate::prelude::*;

use alloc::collections::BTreeMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanStep {
    pub action: String,
    pub args: BTreeMap<String, f64>,
    pub rationale: String,
    pub estimated_risk: f64,
    pub estimated_progress: f64,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    pub horizon: usize,
    pub steps: Vec<PlanStep>,
    pub total_score: f64,
}

#[derive(Debug, Clone)]
pub struct ShortHorizonPlanner {
    pub horizon: usize,
    pub progress_weight: f64,
    pub risk_weight: f64,
}

impl Default for ShortHorizonPlanner {
    fn default() -> Self {
        Self {
            horizon: 3,
            progress_weight: 1.0,
            risk_weight: 0.45,
        }
    }
}

impl ShortHorizonPlanner {
    pub fn new(horizon: usize) -> Self {
        Self {
            horizon: horizon.max(1),
            ..Self::default()
        }
    }

    pub fn plan(
        &self,
        state: &BTreeMap<String, f64>,
        goal: &BTreeMap<String, f64>,
        candidates: &[(String, f64)],
        stall: bool,
    ) -> Plan {
        let mut sim = state.clone();
        let mut steps = Vec::new();
        let mut total_score = 0.0;
        let blend = if stall { 0.45 } else { 0.30 };

        for _ in 0..self.horizon {
            let mut best: Option<PlanStep> = None;
            for (action, risk) in candidates {
                let mut next_state = sim.clone();
                let mut args = BTreeMap::new();
                let mut dist_before = 0.0;
                let mut dist_after = 0.0;

                for (k, target) in goal {
                    let cur = sim.get(k).copied().unwrap_or(0.0);
                    let next_val = cur + blend * (target - cur);
                    dist_before += (cur - target).abs();
                    dist_after += (next_val - target).abs();
                    args.insert(k.clone(), next_val);
                    next_state.insert(k.clone(), next_val);
                }

                let progress = (dist_before - dist_after).max(0.0);
                let score = self.progress_weight * progress - self.risk_weight * (*risk);
                let replace = best.as_ref().map(|b| score > b.score).unwrap_or(true);
                if replace {
                    best = Some(PlanStep {
                        action: action.clone(),
                        args,
                        rationale: if stall {
                            "stall-explore".into()
                        } else {
                            "greedy".into()
                        },
                        estimated_risk: *risk,
                        estimated_progress: progress,
                        score,
                    });
                }
            }
            if let Some(step) = best {
                for (k, v) in &step.args {
                    sim.insert(k.clone(), *v);
                }
                total_score += step.score;
                steps.push(step);
            } else {
                break;
            }
        }

        Plan {
            horizon: steps.len(),
            steps,
            total_score,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planner_proposes_horizon_steps_without_executing() {
        let planner = ShortHorizonPlanner::new(2);
        let mut state = BTreeMap::new();
        state.insert("x".into(), 0.0);
        let mut goal = BTreeMap::new();
        goal.insert("x".into(), 1.0);
        let candidates = vec![("measure".into(), 0.1), ("risky".into(), 0.8)];

        let plan = planner.plan(&state, &goal, &candidates, false);
        assert_eq!(plan.horizon, 2);
        assert_eq!(plan.steps[0].action, "measure");
        assert!(plan.total_score > 0.0);
    }
}
