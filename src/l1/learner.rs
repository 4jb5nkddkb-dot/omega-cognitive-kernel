// Learner logic is currently folded into L1CognitiveLayer::process
// for simpler ownership. This module is kept as extension point.
use crate::error::Result;
use crate::memory::MemoryFacade;
use crate::types::Trajectory;

pub struct Learner;

impl Learner {
    pub fn update(memory: &mut MemoryFacade, traj: &Trajectory) -> Result<()> {
        memory.update(traj)
    }
}
