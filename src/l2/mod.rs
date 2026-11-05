pub mod emergence;
pub mod fixed_point;
pub mod modulator;
pub mod monitor;

use crate::error::Result;
use crate::types::{MetaMetrics, RuleEffect, SystemState};
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
    pub exited_fixed_point: bool,
    pub alignment_score: f64,
    /// Soft-updated RuleEffect after L2 modulation (if any).
    pub effect_nudge: Option<RuleEffect>,
}

pub struct L2MetaLayer {
    pub monitor: MetaMonitor,
    pub detector: EmergenceDetector,
    pub fixed_point: OmegaDayanFixedPoint,
    pub modulator: MetaModulator,
    pub in_fixed_point: bool,
    /// How many consecutive steps we have been inside the fixed point.
    pub dwell_steps: u64,
    pub last_alignment_score: f64,
}

impl L2MetaLayer {
    pub fn new() -> Self {
        Self {
            monitor: MetaMonitor,
            detector: EmergenceDetector::default(),
            fixed_point: OmegaDayanFixedPoint,
            modulator: MetaModulator,
            in_fixed_point: false,
            dwell_steps: 0,
            last_alignment_score: 0.0,
        }
    }

    pub fn step(
        &mut self,
        state: &SystemState,
        biases: HashMap<String, f64>,
        constraints: HashMap<String, f64>,
        current_effect: &RuleEffect,
    ) -> Result<L2Result> {
        let metrics = self.monitor.evaluate(state)?;

        let mut new_biases = biases;
        let mut new_constraints = constraints;
        let mut entered = false;
        let mut exited = false;
        let mut alignment_score = self.last_alignment_score;
        let mut effect_nudge = None;

        if self.in_fixed_point {
            self.dwell_steps = self.dwell_steps.saturating_add(1);
            if self.detector.should_exit(&metrics) || self.dwell_steps > 40 {
                self.in_fixed_point = false;
                self.dwell_steps = 0;
                exited = true;
            } else if self.dwell_steps % 6 == 0 {
                let fp = self.fixed_point.enter(state)?;
                alignment_score = fp.score;
                self.last_alignment_score = fp.score;
                let (b, c) = self.modulator.apply(&fp, new_biases, new_constraints);
                new_biases = b;
                new_constraints = c;
                effect_nudge = Some(self.modulator.apply_to_effect(&fp, current_effect));
            }
        } else if self.detector.should_trigger(&metrics) {
            let fp = self.fixed_point.enter(state)?;
            entered = fp.aligned;
            self.in_fixed_point = fp.aligned;
            if fp.aligned {
                self.dwell_steps = 1;
            }
            alignment_score = fp.score;
            self.last_alignment_score = fp.score;
            let (b, c) = self.modulator.apply(&fp, new_biases, new_constraints);
            new_biases = b;
            new_constraints = c;
            effect_nudge = Some(self.modulator.apply_to_effect(&fp, current_effect));
        }

        Ok(L2Result {
            metrics,
            in_fixed_point: self.in_fixed_point,
            biases: new_biases,
            constraints: new_constraints,
            entered_fixed_point: entered,
            exited_fixed_point: exited,
            alignment_score,
            effect_nudge,
        })
    }
}

impl Default for L2MetaLayer {
    fn default() -> Self {
        Self::new()
    }
}
