use pyo3::prelude::*;
use crate::types::{AuditEvent, AuditRecord, AuditSummary};
use crate::error::{PythonBindingError, PyResult};
use std::path::Path;

/// Python wrapper for AuditLog
#[pyclass]
pub struct AuditLog {
    path: String,
}

#[pymethods]
impl AuditLog {
    /// Create an AuditLog from a state file path
    #[staticmethod]
    pub fn beside_state(state_path: &str) -> PyResult<Self> {
        let path = Path::new(state_path)
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("audit.jsonl");
        Ok(Self {
            path: path.to_string_lossy().to_string(),
        })
    }

    /// Read all audit records
    pub fn read_all(&self) -> PyResult<Vec<AuditRecord>> {
        use std::fs;
        if !Path::new(&self.path).exists() {
            return Ok(vec![]);
        }
        let data = fs::read_to_string(&self.path)
            .map_err(PythonBindingError::Io)?;
        let rows: Vec<AuditRecord> = data
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        Ok(rows)
    }

    /// Get the last n audit records
    pub fn tail(&self, n: usize) -> PyResult<Vec<AuditRecord>> {
        let mut rows = self.read_all()?;
        if rows.len() > n {
            rows = rows.split_off(rows.len() - n);
        }
        Ok(rows)
    }

    /// Get current file length in bytes (for polling)
    pub fn byte_len(&self) -> PyResult<u64> {
        use std::fs;
        if !Path::new(&self.path).exists() {
            return Ok(0);
        }
        let meta = fs::metadata(&self.path).map_err(PythonBindingError::Io)?;
        Ok(meta.len())
    }

    /// Poll for new records after offset
    pub fn poll_after(&self, offset: u64) -> PyResult<(Vec<AuditRecord>, u64)> {
        use std::fs;
        use std::io::{Read, Seek, SeekFrom};

        if !Path::new(&self.path).exists() {
            return Ok((vec![], 0));
        }
        let mut f = fs::File::open(&self.path).map_err(PythonBindingError::Io)?;
        let len = f.metadata().map_err(PythonBindingError::Io)?.len();
        if offset > len {
            // File was truncated/rotated — resync from start.
            return self.poll_after(0);
        }
        f.seek(SeekFrom::Start(offset)).map_err(PythonBindingError::Io)?;
        let mut buf = String::new();
        f.read_to_string(&mut buf).map_err(PythonBindingError::Io)?;

        let mut rows = Vec::new();
        let mut consumed = 0usize;
        for line in buf.split_inclusive('\n') {
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                consumed += line.len();
                continue;
            }
            // Incomplete last line (no newline yet) — wait for more bytes.
            if !line.ends_with('\n') {
                break;
            }
            if let Ok(rec) = serde_json::from_str::<AuditRecord>(trimmed) {
                rows.push(rec);
            }
            consumed += line.len();
        }
        let new_offset = offset + consumed as u64;
        Ok((rows, new_offset))
    }

    /// Generate summary statistics
    pub fn summarize(&self) -> PyResult<AuditSummary> {
        let rows = self.read_all()?;
        use crate::types::AuditSummary;
        let mut s = AuditSummary {
            total_events: rows.len(),
            ..AuditSummary::default()
        };
        use std::collections::HashMap;
        let mut actions: HashMap<String, usize> = HashMap::new();
        if let Some(first) = rows.first() {
            s.first_ts = Some(first.ts.clone());
        }
        if let Some(last) = rows.last() {
            s.last_ts = Some(last.ts.clone());
        }
        for r in rows {
            match &r.event {
                AuditEvent::Step { action, avg_err, damping, .. } => {
                    s.steps += 1;
                    *actions.entry(action.clone()).or_insert(0) += 1;
                    s.last_action = Some(action.clone());
                    s.last_avg_err = Some(*avg_err);
                    s.last_damping = Some(*damping);
                }
                AuditEvent::PatchAccepted { .. } => s.patches_accepted += 1,
                AuditEvent::PatchRejected { .. } => s.patches_rejected += 1,
                AuditEvent::Checkpoint { .. } => s.checkpoints += 1,
                AuditEvent::Rollback { .. } => s.rollbacks += 1,
                AuditEvent::TaskEpisode { .. } => s.task_episodes += 1,
                AuditEvent::ToolAllowed { tool } => {
                    s.tools_allowed += 1;
                    *actions.entry(format!("tool:{tool}")).or_insert(0) += 1;
                }
                AuditEvent::ToolDenied { tool, .. } => {
                    s.tools_denied += 1;
                    *actions.entry(format!("deny:{tool}")).or_insert(0) += 1;
                }
                AuditEvent::FixedPointEntered { .. } => s.fixed_point_enters += 1,
                AuditEvent::FixedPointExited { .. } => s.fixed_point_exits += 1,
                AuditEvent::L2ModulationProposed { .. } => s.l2_modulation_proposals += 1,
                AuditEvent::L2ModulationCommitted { .. } => s.l2_modulation_commits += 1,
            }
        }
        let mut counts: Vec<_> = actions.into_iter().collect();
        counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        s.action_counts = counts;
        Ok(s)
    }
}