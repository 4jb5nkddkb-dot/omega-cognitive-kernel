use crate::error::Result;
use crate::l1::sandbox::ValidationReport;
use crate::types::CognitivePatch;
use std::collections::HashMap;

pub const MAX_HISTORY: usize = 64;

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
        self.history.push((patch.clone(), report.message.clone()));
        // FIFO bound
        if self.history.len() > MAX_HISTORY {
            let excess = self.history.len() - MAX_HISTORY;
            self.history.drain(0..excess);
        }
        self.active.insert(patch.target_module.clone(), patch.code);
        Ok(())
    }

    pub fn snapshot(&self) -> (HashMap<String, String>, Vec<(CognitivePatch, String)>) {
        (self.active.clone(), self.history.clone())
    }

    pub fn restore(
        active: HashMap<String, String>,
        history: Vec<(CognitivePatch, String)>,
    ) -> Self {
        let mut h = history;
        if h.len() > MAX_HISTORY {
            let excess = h.len() - MAX_HISTORY;
            h.drain(0..excess);
        }
        Self { active, history: h }
    }
}

impl Default for VersionedStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_patch(id: u8) -> CognitivePatch {
        CognitivePatch {
            patch_id: Uuid::new_v4(),
            description: format!("p{id}"),
            code: format!("// {id}"),
            target_module: "cognitive_rules".into(),
            expected_effect: "test".into(),
            risk_level: 0.1,
            created_at: Utc::now(),
            effect: crate::types::RuleEffect::default(),
        }
    }

    #[test]
    fn history_is_bounded() {
        let mut store = VersionedStore::new();
        let report = ValidationReport {
            accepted: true,
            message: "ok".into(),
            patch_id: Uuid::new_v4(),
        };
        for i in 0..80 {
            store.commit(make_patch(i as u8), &report).unwrap();
        }
        assert!(store.history.len() <= MAX_HISTORY);
        assert_eq!(store.history.len(), MAX_HISTORY);
    }
}
