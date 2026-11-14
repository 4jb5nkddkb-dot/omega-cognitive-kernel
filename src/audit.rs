//! Append-only audit log (S3 hardening).
//!
//! `AuditEvent` is the kernel's public event stream. Prefer the alias
//! [`KernelEvent`] at the crate root when wiring integrators.

use crate::error::{OmegaError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Public event stream for integrators (same shape as [`AuditEvent`]).
pub type KernelEvent = AuditEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuditEvent {
    Step {
        step: u64,
        goal: String,
        action: String,
        avg_err: f64,
        damping: f64,
    },
    PatchAccepted {
        step: u64,
        patch_id: Uuid,
        damping: f64,
        description: String,
    },
    PatchRejected {
        step: u64,
        reason: String,
    },
    Checkpoint {
        checkpoint_id: Uuid,
        step: u64,
        label: String,
    },
    Rollback {
        checkpoint_id: Uuid,
        step_before: u64,
    },
    TaskEpisode {
        task: String,
        reward: f64,
        steps: u32,
    },
    /// Tool-call gate: proposed tool was on allow-list and recorded.
    ToolAllowed {
        tool: String,
    },
    /// Tool-call gate: proposed tool blocked (deny-list or not allowed).
    ToolDenied {
        tool: String,
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub ts: DateTime<Utc>,
    pub event: AuditEvent,
}

/// Operator-facing rollup of an audit.jsonl file.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AuditSummary {
    pub total_events: usize,
    pub steps: usize,
    pub patches_accepted: usize,
    pub patches_rejected: usize,
    pub checkpoints: usize,
    pub rollbacks: usize,
    pub task_episodes: usize,
    pub tools_allowed: usize,
    pub tools_denied: usize,
    pub first_ts: Option<DateTime<Utc>>,
    pub last_ts: Option<DateTime<Utc>>,
    pub last_action: Option<String>,
    pub last_avg_err: Option<f64>,
    pub last_damping: Option<f64>,
    pub action_counts: Vec<(String, usize)>,
}

impl AuditSummary {
    pub fn from_records(rows: &[AuditRecord]) -> Self {
        use std::collections::HashMap;
        let mut s = AuditSummary {
            total_events: rows.len(),
            ..AuditSummary::default()
        };
        let mut actions: HashMap<String, usize> = HashMap::new();
        if let Some(first) = rows.first() {
            s.first_ts = Some(first.ts);
        }
        if let Some(last) = rows.last() {
            s.last_ts = Some(last.ts);
        }
        for r in rows {
            match &r.event {
                AuditEvent::Step {
                    action,
                    avg_err,
                    damping,
                    ..
                } => {
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
            }
        }
        let mut counts: Vec<_> = actions.into_iter().collect();
        counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        s.action_counts = counts;
        s
    }
}

pub struct AuditLog {
    pub path: PathBuf,
}

impl AuditLog {
    pub fn beside_state(state_path: &Path) -> Self {
        let path = state_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("audit.jsonl");
        Self { path }
    }

    pub fn append(&self, event: AuditEvent) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| OmegaError::Persist(e.to_string()))?;
        }
        let rec = AuditRecord {
            ts: Utc::now(),
            event,
        };
        let line = serde_json::to_string(&rec).map_err(|e| OmegaError::Persist(e.to_string()))?;
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| OmegaError::Persist(e.to_string()))?;
        writeln!(f, "{line}").map_err(|e| OmegaError::Persist(e.to_string()))?;
        Ok(())
    }

    pub fn tail(&self, n: usize) -> Result<Vec<AuditRecord>> {
        let mut rows = self.read_all()?;
        if rows.len() > n {
            rows = rows.split_off(rows.len() - n);
        }
        Ok(rows)
    }

    pub fn read_all(&self) -> Result<Vec<AuditRecord>> {
        if !self.path.exists() {
            return Ok(vec![]);
        }
        let data =
            fs::read_to_string(&self.path).map_err(|e| OmegaError::Persist(e.to_string()))?;
        let rows: Vec<AuditRecord> = data
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        Ok(rows)
    }

    /// Current file length in bytes (0 if missing). Use as the cursor for [`poll_after`].
    pub fn byte_len(&self) -> Result<u64> {
        if !self.path.exists() {
            return Ok(0);
        }
        let meta = fs::metadata(&self.path).map_err(|e| OmegaError::Persist(e.to_string()))?;
        Ok(meta.len())
    }

    /// Read complete JSONL records written after `offset` bytes.
    ///
    /// Returns `(new_records, new_offset)`. Incomplete trailing lines are left
    /// for a later poll (offset does not advance past them).
    pub fn poll_after(&self, offset: u64) -> Result<(Vec<AuditRecord>, u64)> {
        if !self.path.exists() {
            return Ok((vec![], 0));
        }
        let mut f = fs::File::open(&self.path).map_err(|e| OmegaError::Persist(e.to_string()))?;
        let len = f
            .metadata()
            .map_err(|e| OmegaError::Persist(e.to_string()))?
            .len();
        if offset > len {
            // File was truncated/rotated — resync from start.
            return self.poll_after(0);
        }
        f.seek(SeekFrom::Start(offset))
            .map_err(|e| OmegaError::Persist(e.to_string()))?;
        let mut buf = String::new();
        f.read_to_string(&mut buf)
            .map_err(|e| OmegaError::Persist(e.to_string()))?;

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

    pub fn summarize(&self) -> Result<AuditSummary> {
        let rows = self.read_all()?;
        Ok(AuditSummary::from_records(&rows))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn append_and_tail() {
        let path = temp_dir().join(format!("omega-audit-{}.jsonl", Uuid::new_v4()));
        let log = AuditLog { path: path.clone() };
        log.append(AuditEvent::Step {
            step: 1,
            goal: "g".into(),
            action: "observe".into(),
            avg_err: 0.5,
            damping: 1.0,
        })
        .unwrap();
        let rows = log.tail(10).unwrap();
        assert_eq!(rows.len(), 1);
        let sum = log.summarize().unwrap();
        assert_eq!(sum.steps, 1);
        assert_eq!(sum.action_counts[0].0, "observe");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn poll_after_reads_only_new_lines() {
        let path = temp_dir().join(format!("omega-poll-{}.jsonl", Uuid::new_v4()));
        let log = AuditLog { path: path.clone() };
        log.append(AuditEvent::Step {
            step: 1,
            goal: "g".into(),
            action: "explore".into(),
            avg_err: 0.4,
            damping: 1.0,
        })
        .unwrap();
        let (first, off) = log.poll_after(0).unwrap();
        assert_eq!(first.len(), 1);
        assert!(off > 0);

        let (none, off2) = log.poll_after(off).unwrap();
        assert!(none.is_empty());
        assert_eq!(off2, off);

        log.append(AuditEvent::PatchAccepted {
            step: 2,
            patch_id: Uuid::nil(),
            damping: 0.8,
            description: "t".into(),
        })
        .unwrap();
        let (second, off3) = log.poll_after(off).unwrap();
        assert_eq!(second.len(), 1);
        assert!(matches!(second[0].event, AuditEvent::PatchAccepted { .. }));
        assert!(off3 > off);

        let _ = fs::remove_file(&path);
    }
}
