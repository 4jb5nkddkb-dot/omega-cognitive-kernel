//! Public product API (S2).
//!
//! Prefer `Kernel` over wiring L0/L1/L2 by hand.

use crate::error::Result;
use crate::l0::decision::ConstrainedDecisionMaker;
use crate::l0::executor::SandboxedExecutor;
use crate::l0::observer::MultiModalObserver;
use crate::l0::predictor::ModelBasedPredictor;
use crate::l0::L0WorldLayer;
use crate::l1::L1CognitiveLayer;
use crate::l2::L2MetaLayer;
use crate::memory::MemoryFacade;
use crate::orchestrator::{OmegaOrchestrator, StepResult};
use crate::persist::{resolve_state_path, KernelSnapshot, DEFAULT_STATE_PATH};
use crate::safety;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

pub type DefaultKernel = Kernel;

pub struct Kernel {
    pub inner: OmegaOrchestrator<
        MultiModalObserver,
        ModelBasedPredictor,
        ConstrainedDecisionMaker,
        SandboxedExecutor,
    >,
    pub state_path: PathBuf,
    pub autosave: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct KernelStatus {
    pub total_steps: u64,
    pub avg_prediction_error: f64,
    pub adaptation_score: f64,
    pub error_damping: f64,
    pub active_modules: usize,
    pub strategy_count: usize,
    pub case_count: usize,
    pub state_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunReport {
    pub steps: u64,
    pub patches_accepted: u64,
    pub early_avg_err: f64,
    pub final_avg_err: f64,
    pub final_damping: f64,
    pub error_decreased: bool,
    pub last_actions: Vec<String>,
}

impl Kernel {
    pub fn new() -> Self {
        Self::with_state_path(DEFAULT_STATE_PATH)
    }

    pub fn with_state_path(path: impl Into<PathBuf>) -> Self {
        let l0 = L0WorldLayer {
            observer: MultiModalObserver::new(),
            predictor: ModelBasedPredictor,
            decision: ConstrainedDecisionMaker,
            executor: SandboxedExecutor::new(vec![
                "noop".into(),
                "observe".into(),
                "explore".into(),
                "exploit".into(),
                "stabilize".into(),
            ]),
            memory: MemoryFacade::new(),
        };
        Self {
            inner: OmegaOrchestrator {
                l0,
                l1: L1CognitiveLayer::new(),
                l2: L2MetaLayer::new(),
                biases: HashMap::new(),
                constraints: safety::default_constraints(),
                total_steps: 0,
            },
            state_path: path.into(),
            autosave: true,
        }
    }

    pub fn open(path: Option<&str>) -> Result<Self> {
        let path = resolve_state_path(path);
        if path.exists() {
            let mut k = Self::with_state_path(path.clone());
            k.load()?;
            Ok(k)
        } else {
            Ok(Self::with_state_path(path))
        }
    }

    pub fn step(&mut self, goal: &str) -> Result<StepResult> {
        let result = self.inner.run_once(goal)?;
        if self.autosave {
            let _ = self.save();
        }
        Ok(result)
    }

    pub fn run(&mut self, goal: &str, steps: u32) -> Result<RunReport> {
        let mut patches = 0u64;
        let mut actions = Vec::new();
        let mut early = 0.0;
        for i in 0..steps {
            let r = self.step(goal)?;
            if i == 0 {
                early = self.inner.l0.memory.cas.avg_prediction_error;
            }
            if r.l1.patched {
                patches += 1;
            }
            if let Some(a) = r.trajectory.actions.first() {
                actions.push(a.name.clone());
            }
        }
        let final_err = self.inner.l0.memory.cas.avg_prediction_error;
        Ok(RunReport {
            steps: steps as u64,
            patches_accepted: patches,
            early_avg_err: early,
            final_avg_err: final_err,
            final_damping: self.inner.l0.memory.cognitive_effect.error_damping,
            error_decreased: final_err < early,
            last_actions: actions,
        })
    }

    pub fn status(&self) -> KernelStatus {
        let cases = self.inner.l0.memory.case.retrieve("", 10_000).len();
        let strategies = self.inner.l0.memory.strategy.retrieve("", 10_000).len();
        KernelStatus {
            total_steps: self.inner.total_steps,
            avg_prediction_error: self.inner.l0.memory.cas.avg_prediction_error,
            adaptation_score: self.inner.l0.memory.cas.adaptation_score,
            error_damping: self.inner.l0.memory.cognitive_effect.error_damping,
            active_modules: self.inner.l1.store.active.len(),
            strategy_count: strategies,
            case_count: cases,
            state_path: self.state_path.display().to_string(),
        }
    }

    pub fn snapshot(&self) -> KernelSnapshot {
        let (cases, cap) = self.inner.l0.memory.case.snapshot();
        let (active, history) = self.inner.l1.store.snapshot();
        KernelSnapshot {
            version: 1,
            total_steps: self.inner.total_steps,
            biases: self.inner.biases.clone(),
            constraints: self.inner.constraints.clone(),
            cas_total_steps: self.inner.l0.memory.cas.total_steps,
            cas_avg_prediction_error: self.inner.l0.memory.cas.avg_prediction_error,
            cas_adaptation_score: self.inner.l0.memory.cas.adaptation_score,
            cognitive_effect: self.inner.l0.memory.cognitive_effect.clone(),
            cases,
            case_capacity: cap,
            strategies: self.inner.l0.memory.strategy.snapshot(),
            active_modules: active,
            patch_history: history,
        }
    }

    pub fn apply_snapshot(&mut self, snap: KernelSnapshot) {
        self.inner.total_steps = snap.total_steps;
        self.inner.biases = snap.biases;
        self.inner.constraints = snap.constraints;
        self.inner.l0.memory.cas.total_steps = snap.cas_total_steps;
        self.inner.l0.memory.cas.avg_prediction_error = snap.cas_avg_prediction_error;
        self.inner.l0.memory.cas.adaptation_score = snap.cas_adaptation_score;
        self.inner.l0.memory.cognitive_effect = snap.cognitive_effect;
        self.inner.l0.memory.case =
            crate::memory::case::CaseMemory::restore(snap.cases, snap.case_capacity);
        self.inner.l0.memory.strategy =
            crate::memory::strategy::StrategyMemory::restore(snap.strategies);
        self.inner.l1.store =
            crate::l1::store::VersionedStore::restore(snap.active_modules, snap.patch_history);
    }

    pub fn save(&self) -> Result<()> {
        self.snapshot().save_to(&self.state_path)
    }

    pub fn load(&mut self) -> Result<()> {
        let snap = KernelSnapshot::load_from(&self.state_path)?;
        self.apply_snapshot(snap);
        Ok(())
    }

    pub fn reset(&mut self) -> Result<()> {
        let path = self.state_path.clone();
        let autosave = self.autosave;
        *self = Self::with_state_path(path);
        self.autosave = autosave;
        if self.state_path.exists() {
            std::fs::remove_file(&self.state_path)
                .map_err(|e| crate::error::OmegaError::Persist(e.to_string()))?;
        }
        Ok(())
    }
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;
    use uuid::Uuid;

    #[test]
    fn run_and_persist_roundtrip() {
        let path = temp_dir().join(format!("omega-api-{}.json", Uuid::new_v4()));
        let mut k = Kernel::with_state_path(&path);
        k.autosave = true;
        let report = k.run("coherent self-model", 5).unwrap();
        assert_eq!(report.steps, 5);
        assert!(path.exists());

        let mut k2 = Kernel::with_state_path(&path);
        k2.load().unwrap();
        assert_eq!(k2.inner.total_steps, 5);
        assert!(k2.status().case_count > 0);
        let _ = std::fs::remove_file(&path);
    }
}
