use crate::error::Result;
use crate::l1::sandbox::ValidationReport;
use crate::types::CognitivePatch;
use std::collections::HashMap;

pub struct VersionedStore {
    pub history: Vec<(CognitivePatch, String)>,
    pub active: HashMap<String, String>,
}

impl VersionedStore {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            active: HashMap::new(),
        }
    }

    pub fn commit(&mut self, patch: CognitivePatch, report: &ValidationReport) -> Result<()> {
        self.history
            .push((patch.clone(), report.message.clone()));
        self.active
            .insert(patch.target_module.clone(), patch.code);
        Ok(())
    }
}

impl Default for VersionedStore {
    fn default() -> Self {
        Self::new()
    }
}
