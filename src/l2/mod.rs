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
    pub candidate_score: f64,
    pub modulation_proposed: bool,
    pub modulation_committed: bool,
    pub modulation_gain: f64,
    /// Soft-updated RuleEffect after L2 modulation (if any).
    pub effect_nudge: Option<RuleEffect>,
}

pub struct L2MetaLayer {
    pub monitor: MetaMonitor,
    pub detector: EmergenceDetector,
    pub fixed_point: OmegaDayanFixedPoint,
    pub modulator: MetaModulator,
    /// Whether L2 is allowed to change biases, constraints, or RuleEffect.
    /// The monitor still runs when disabled so benchmark baselines retain
    /// comparable observability without receiving L2 interventions.
    pub enabled: bool,
    pub in_fixed_point: bool,
    /// How many consecutive steps we have been inside the fixed point.
    pub dwell_steps: u64,
    pub last_alignment_score: f64,
    pub commit_threshold: f64,
    pub commit_cooldown_steps: u64,
    pub commit_cooldown_remaining: u64,
    pub last_consequence_score: f64,
}

impl L2MetaLayer {
    pub fn new() -> Self {
        Self {
            monitor: MetaMonitor,
            detector: EmergenceDetector::default(),
            fixed_point: OmegaDayanFixedPoint,
            modulator: MetaModulator,
            enabled: true,
            in_fixed_point: false,
            dwell_steps: 0,
            last_alignment_score: 0.0,
            commit_threshold: 0.82,
            commit_cooldown_steps: 5,
            commit_cooldown_remaining: 0,
            last_consequence_score: 0.0,
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
        let mut candidate_score = 0.0;
        let mut modulation_proposed = false;
        let mut modulation_committed = false;
        let mut modulation_gain = 0.0;

        self.commit_cooldown_remaining = self.commit_cooldown_remaining.saturating_sub(1);

        if !self.enabled {
            self.in_fixed_point = false;
            self.dwell_steps = 0;
            return Ok(L2Result {
                metrics,
                in_fixed_point: false,
                biases: new_biases,
                constraints: new_constraints,
                entered_fixed_point: false,
                exited_fixed_point: false,
                alignment_score: 0.0,
                candidate_score,
                modulation_proposed,
                modulation_committed,
                modulation_gain,
                effect_nudge: None,
            });
        }

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
                candidate_score = fp.score;
                modulation_proposed = true;
                if fp.score >= self.commit_threshold && self.commit_cooldown_remaining == 0 {
                    modulation_committed = true;
                    modulation_gain = ((fp.score - self.commit_threshold)
                        / (1.0 - self.commit_threshold))
                        .clamp(0.10, 1.0);
                    let (b, c) = self.modulator.apply_scaled(
                        &fp,
                        modulation_gain,
                        new_biases,
                        new_constraints,
                    );
                    new_biases = b;
                    new_constraints = c;
                    effect_nudge = Some(self.modulator.apply_to_effect_scaled(
                        &fp,
                        modulation_gain,
                        current_effect,
                    ));
                    self.commit_cooldown_remaining = self.commit_cooldown_steps;
                }
            }
        } else if self.detector.should_trigger(&metrics) {
            let fp = self.fixed_point.enter(state)?;
            candidate_score = fp.score;
            modulation_proposed = true;
            alignment_score = fp.score;
            self.last_alignment_score = fp.score;
            if fp.score >= self.commit_threshold && self.commit_cooldown_remaining == 0 {
                modulation_committed = true;
                modulation_gain = ((fp.score - self.commit_threshold)
                    / (1.0 - self.commit_threshold))
                    .clamp(0.10, 1.0);
                let (b, c) =
                    self.modulator
                        .apply_scaled(&fp, modulation_gain, new_biases, new_constraints);
                new_biases = b;
                new_constraints = c;
                effect_nudge = Some(self.modulator.apply_to_effect_scaled(
                    &fp,
                    modulation_gain,
                    current_effect,
                ));
                self.commit_cooldown_remaining = self.commit_cooldown_steps;
                entered = fp.aligned;
                self.in_fixed_point = fp.aligned;
                if fp.aligned {
                    self.dwell_steps = 1;
                }
            }
        }

        Ok(L2Result {
            metrics,
            in_fixed_point: self.in_fixed_point,
            biases: new_biases,
            constraints: new_constraints,
            entered_fixed_point: entered,
            exited_fixed_point: exited,
            alignment_score,
            candidate_score,
            modulation_proposed,
            modulation_committed,
            modulation_gain,
            effect_nudge,
        })
    }

    /// Feed back the short-term consequence of the last committed modulation.
    /// Negative outcomes extend the commit cooldown and leave fixed-point mode.
    pub fn observe_consequence(&mut self, reward_delta: f64, error_delta: f64, risk_delta: f64) {
        let score = 0.60 * reward_delta - 0.25 * error_delta - 0.15 * risk_delta;
        self.last_consequence_score = score;
        if score < -0.05 {
            self.commit_cooldown_remaining = self
                .commit_cooldown_remaining
                .max(self.commit_cooldown_steps.saturating_mul(2));
            self.in_fixed_point = false;
            self.dwell_steps = 0;
        }
    }
}

impl Default for L2MetaLayer {
    fn default() -> Self {
        Self::new()
    }
}
