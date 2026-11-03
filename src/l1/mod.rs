pub mod abstractor;
pub mod learner;
pub mod reflector;
pub mod patch;
pub mod sandbox;
pub mod store;

use crate::error::Result;
use crate::memory::MemoryFacade;
use crate::types::Trajectory;
use abstractor::Abstractor;
use reflector::Reflector;
use sandbox::CodeSandbox;
use store::VersionedStore;

#[derive(Debug)]
pub struct L1Result {
    pub patched: bool,
    pub patch_id: Option<uuid::Uuid>,
    pub reason: Option<String>,
}

pub struct L1CognitiveLayer {
    pub abstractor: Abstractor,
    pub reflector: Reflector,
    pub sandbox: CodeSandbox,
    pub store: VersionedStore,
}

impl L1CognitiveLayer {
    pub fn new() -> Self {
        Self {
            abstractor: Abstractor,
            reflector: Reflector::default(),
            sandbox: CodeSandbox::default(),
            store: VersionedStore::new(),
        }
    }

    pub fn process(
        &mut self,
        trajectory: &Trajectory,
        memory: &mut MemoryFacade,
    ) -> Result<L1Result> {
        let abstraction = self.abstractor.distill(trajectory)?;
        memory.update(trajectory)?;

        if let Some(patch) = self.reflector.reflect(trajectory, &abstraction)? {
            let report = self.sandbox.validate(&patch)?;
            if report.accepted {
                self.store.commit(patch, &report)?;
                return Ok(L1Result {
                    patched: true,
                    patch_id: Some(report.patch_id),
                    reason: None,
                });
            } else {
                return Ok(L1Result {
                    patched: false,
                    patch_id: None,
                    reason: Some(report.message),
                });
            }
        }

        Ok(L1Result {
            patched: false,
            patch_id: None,
            reason: None,
        })
    }
}

impl Default for L1CognitiveLayer {
    fn default() -> Self {
        Self::new()
    }
}
