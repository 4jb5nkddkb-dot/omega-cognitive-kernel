pub mod emergence;
pub mod fixed_point;
pub mod modulator;
pub mod monitor;

use crate::error::Result;
use crate::types::{MetaMetrics, SystemState};
use emergence::EmergenceDetector;
use fixed_point::OmegaDayanFixedPoint;
use modulator::MetaModulator;
use monitor::MetaMonitor;
use std::collections::HashMap;

#[derive(Debug)]
pub struct L2Result {
    pub metrics: MetaMetrics,
    pub in_fixed_point: bool,
    pub biases: HashMap<String, f64>,
    pub constraints: HashMap<String, f64>,
    pub entered_fixed_point: bool,
    pub alignment_score: f64,
}

pub struct L2MetaLayer {
    pub monitor: MetaMonitor,
    pub detector: EmergenceDetector,
    pub fixed_point: OmegaDayanFixedPoint,
    pub modulator: MetaModulator,
    pub in_fixed_point: bool,
}

impl L2MetaLayer {
    pub fn new() -> Self {
        Self {
            monitor: MetaMonitor,
            detector: EmergenceDetector::default(),
            fixed_point: OmegaDayanFixedPoint,
            modulator: MetaModulator,
            in_fixed_point: false,
        }
    }

    pub fn step(
        &mut self,
        state: &SystemState,
        biases: HashMap<String, f64>,
        constraints: HashMap<String, f64>,
    ) -> Result<L2Result> {
        let metrics = self.monitor.evaluate(state)?;

        let mut new_biases = biases;
        let mut new_constraints = constraints;
        let mut entered = false;
        let mut alignment_score = 0.0;

        if self.detector.should_trigger(&metrics) {
            let fp = self.fixed_point.enter(state)?;
            entered = fp.aligned;
            self.in_fixed_point = fp.aligned;
            alignment_score = fp.score;
            let (b, c) = self.modulator.apply(&fp, new_biases, new_constraints);
            new_biases = b;
            new_constraints = c;
        }

        Ok(L2Result {
            metrics,
            in_fixed_point: self.in_fixed_point,
            biases: new_biases,
            constraints: new_constraints,
            entered_fixed_point: entered,
            alignment_score,
        })
    }
}

impl Default for L2MetaLayer {
    fn default() -> Self {
        Self::new()
    }
}
