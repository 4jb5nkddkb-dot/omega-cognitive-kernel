use crate::l2::fixed_point::FixedPointResult;
use crate::types::RuleEffect;
use std::collections::HashMap;

pub struct MetaModulator;

impl MetaModulator {
    pub fn apply(
        &self,
        fp: &FixedPointResult,
        mut biases: HashMap<String, f64>,
        mut constraints: HashMap<String, f64>,
    ) -> (HashMap<String, f64>, HashMap<String, f64>) {
        for (k, v) in &fp.modulation.bias_delta {
            *biases.entry(k.clone()).or_insert(0.0) += *v;
        }
        for (k, v) in &fp.modulation.constraint_delta {
            let entry = constraints.entry(k.clone()).or_insert(0.65);
            *entry = (*entry + *v).clamp(0.1, 0.95);
        }
        (biases, constraints)
    }

    /// Soft-merge fixed-point deltas into a RuleEffect, then sanitize.
    /// Deltas are intentionally small so L1 patches remain the primary policy writer.
    pub fn apply_to_effect(&self, fp: &FixedPointResult, effect: &RuleEffect) -> RuleEffect {
        let mut e = effect.clone();
        e.explore_bonus += fp.modulation.explore_bonus_delta;
        e.risk_tolerance += fp.modulation.risk_tolerance_delta;
        e.sanitize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l2::fixed_point::Modulation;

    #[test]
    fn soft_merge_clamps_via_sanitize() {
        let modu = MetaModulator;
        let fp = FixedPointResult {
            mode: "t".into(),
            aligned: true,
            score: 0.9,
            modulation: Modulation {
                bias_delta: HashMap::new(),
                constraint_delta: HashMap::new(),
                explore_bonus_delta: 5.0, // way out of bounds
                risk_tolerance_delta: -2.0,
            },
        };
        let base = RuleEffect::default();
        let merged = modu.apply_to_effect(&fp, &base);
        assert!(merged.explore_bonus <= 1.5);
        assert!(merged.risk_tolerance >= 0.15);
    }
}
