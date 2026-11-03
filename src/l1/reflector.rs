use crate::error::Result;
use crate::l1::abstractor::Abstraction;
use crate::l1::patch::PatchGenerator;
use crate::types::{CognitivePatch, Trajectory};

pub struct Reflector {
    pub error_threshold: f64,
}

impl Default for Reflector {
    fn default() -> Self {
        Self {
            error_threshold: 0.30,
        }
    }
}

impl Reflector {
    pub fn reflect(
        &self,
        traj: &Trajectory,
        abs: &Abstraction,
    ) -> Result<Option<CognitivePatch>> {
        if abs.mean_error < self.error_threshold && abs.action_count > 0 {
            return Ok(None);
        }
        let patch = PatchGenerator::generate(traj, &abs.summary);
        Ok(Some(patch))
    }
}
