use crate::l2::fixed_point::FixedPointResult;
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
}
