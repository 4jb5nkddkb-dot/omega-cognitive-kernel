//! Append-only audit log (S3 hardening).
//!
//! `AuditEvent` is the kernel's public event stream. Prefer the alias
//! [`KernelEvent`] at the crate root when wiring integrators.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::error::{OmegaError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
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
    /// L2 entered the fixed-point regime.
    FixedPointEntered {
        step: u64,
        alignment_score: f64,
    },
    /// L2 exited the fixed-point regime.
    FixedPointExited {
        step: u64,
        alignment_score: f64,
        dwell_steps: u64,
    },
    L2ModulationProposed {
        step: u64,
        score: f64,
    },
    L2ModulationCommitted {
        step: u64,
        score: f64,
        gain: f64,
    },
    ExternalStep {
        step_id: u64,
        prediction_error: f64,
        observed_risk: f64,
        l1_gate: String,
        keep_steady: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        patch_id: Option<String>,
    },
    ExternalRollback {
        step_id: u64,
        checkpoint_id: String,
        consecutive_rollbacks: u32,
        next_cooldown: u64,
    },
    ExternalStepDuplicate {
        step_id: u64,
        last_step_id: u64,
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
    pub fixed_point_enters: usize,
    pub fixed_point_exits: usize,
    pub l2_modulation_proposals: usize,
    pub l2_modulation_commits: usize,
    pub first_ts: Option<DateTime<Utc>>,
    pub last_ts: Option<DateTime<Utc>>,
    pub last_action: Option<String>,
    pub last_avg_err: Option<f64>,
    pub last_damping: Option<f64>,
    pub action_counts: Vec<(String, usize)>,
}

impl AuditSummary {
    pub fn from_records(rows: &[AuditRecord]) -> Self {
        use hashbrown::HashMap;
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
                AuditEvent::FixedPointEntered { .. } => s.fixed_point_enters += 1,
                AuditEvent::FixedPointExited { .. } => s.fixed_point_exits += 1,
                AuditEvent::L2ModulationProposed { .. } => s.l2_modulation_proposals += 1,
                AuditEvent::L2ModulationCommitted { .. } => s.l2_modulation_commits += 1,
                AuditEvent::ExternalStep { .. } => s.steps += 1,
                AuditEvent::ExternalRollback { .. } => s.rollbacks += 1,
                AuditEvent::ExternalStepDuplicate { .. } => {}
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

/// Default maximum size for an audit JSONL file (64 MiB).
///
/// Existing files larger than this remain readable; only new appends are
/// limited. Integrators can choose a different ceiling per append.
pub const DEFAULT_MAX_AUDIT_BYTES: u64 = 64 * 1024 * 1024;

struct AuditAppendLock {
    #[cfg(any(unix, windows))]
    _file: fs::File,
}

impl AuditAppendLock {
    fn acquire(path: &Path) -> Result<Self> {
        #[cfg(any(unix, windows))]
        {
            use fs2::FileExt;

            let file = crate::private_fs::open_options()
                .read(true)
                .write(true)
                .create(true)
                .open(path)
                .map_err(|error| {
                    OmegaError::Persist(format!("audit lock open {}: {error}", path.display()))
                })?;
            file.lock_exclusive().map_err(|error| {
                OmegaError::Persist(format!("audit lock acquire {}: {error}", path.display()))
            })?;
            Ok(Self { _file: file })
        }
        #[cfg(not(any(unix, windows)))]
        {
            // These targets do not support the crate's advisory file locks;
            // single-writer audit usage remains supported there.
            let _ = path;
            Ok(Self {})
        }
    }
}

fn audit_lock_path(path: &Path) -> PathBuf {
    let mut lock_path = path.as_os_str().to_os_string();
    lock_path.push(".lock");
    PathBuf::from(lock_path)
}

impl AuditLog {
    fn malformed_record(&self, line: usize, offset: u64, error: serde_json::Error) -> OmegaError {
        OmegaError::Persist(format!(
            "malformed audit record in {} at line {line}, byte offset {offset}: {error}",
            self.path.display()
        ))
    }

    pub fn beside_state(state_path: &Path) -> Self {
        let path = state_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("audit.jsonl");
        Self { path }
    }

    pub fn append(&self, event: AuditEvent) -> Result<()> {
        self.append_with_max_bytes(event, DEFAULT_MAX_AUDIT_BYTES)
    }

    /// Append an event with an explicit maximum audit-file size in bytes.
    ///
    /// The check and append are serialized across cooperating local writers.
    /// A zero limit rejects every record; use `u64::MAX` for an effectively
    /// unlimited file. Existing bytes are never truncated or rotated.
    pub fn append_with_max_bytes(&self, event: AuditEvent, max_bytes: u64) -> Result<()> {
        self.append_with_privacy_and_max_bytes(event, false, max_bytes)
    }

    /// Append an event, optionally replacing user-provided text fields before serialization.
    pub fn append_with_privacy(&self, event: AuditEvent, redact_user_content: bool) -> Result<()> {
        self.append_with_privacy_and_max_bytes(event, redact_user_content, DEFAULT_MAX_AUDIT_BYTES)
    }

    /// Append an event with privacy redaction and an explicit maximum file size.
    pub fn append_with_privacy_and_max_bytes(
        &self,
        mut event: AuditEvent,
        redact_user_content: bool,
        max_bytes: u64,
    ) -> Result<()> {
        if redact_user_content {
            match &mut event {
                AuditEvent::Step { goal, .. } => *goal = "[REDACTED]".into(),
                AuditEvent::Checkpoint { label, .. } => *label = "[REDACTED]".into(),
                AuditEvent::TaskEpisode { task, .. } => *task = "[REDACTED]".into(),
                _ => {}
            }
        }
        if let Some(parent) = self.path.parent() {
            crate::private_fs::create_dir_all(parent)
                .map_err(|e| OmegaError::Persist(e.to_string()))?;
        }
        let _lock = AuditAppendLock::acquire(&audit_lock_path(&self.path))?;

        let rec = AuditRecord {
            ts: crate::now_utc(),
            event,
        };
        let mut line =
            serde_json::to_string(&rec).map_err(|e| OmegaError::Persist(e.to_string()))?;
        line.push('\n');

        let current_len = match fs::metadata(&self.path) {
            Ok(metadata) => metadata.len(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
            Err(error) => return Err(OmegaError::Persist(error.to_string())),
        };
        let record_len = u64::try_from(line.len())
            .map_err(|error| OmegaError::Persist(format!("audit record size: {error}")))?;
        let resulting_len = current_len.checked_add(record_len);
        if !matches!(resulting_len, Some(size) if size <= max_bytes) {
            return Err(OmegaError::Persist(format!(
                "audit log size limit exceeded for {}: current {current_len} bytes, record {record_len} bytes, maximum {max_bytes} bytes",
                self.path.display()
            )));
        }

        let mut f = crate::private_fs::open_options()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| OmegaError::Persist(e.to_string()))?;
        if let Err(error) = f.write_all(line.as_bytes()) {
            return match f.set_len(current_len) {
                Ok(()) => Err(OmegaError::Persist(error.to_string())),
                Err(rollback_error) => Err(OmegaError::Persist(format!(
                    "audit write failed: {error}; partial-record cleanup failed: {rollback_error}"
                ))),
            };
        }
        Ok(())
    }

    pub fn tail(&self, n: usize) -> Result<Vec<AuditRecord>> {
        let mut rows = self.read_all()?;
        if rows.len() > n {
            rows = rows.split_off(rows.len() - n);
        }
        Ok(rows)
    }

    /// Read and validate every complete JSONL record.
    ///
    /// A malformed trailing line is an error here; use `poll_after` to retain
    /// an incomplete trailing record while a writer is still appending.
    pub fn read_all(&self) -> Result<Vec<AuditRecord>> {
        if !self.path.exists() {
            return Ok(vec![]);
        }
        let data =
            fs::read_to_string(&self.path).map_err(|e| OmegaError::Persist(e.to_string()))?;
        let mut rows = Vec::new();
        let mut offset = 0u64;
        for (index, line) in data.split_inclusive('\n').enumerate() {
            let line_number = index + 1;
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if !trimmed.trim().is_empty() {
                let record = serde_json::from_str(trimmed)
                    .map_err(|error| self.malformed_record(line_number, offset, error))?;
                rows.push(record);
            }
            offset += line.len() as u64;
        }
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
        for (index, line) in buf.split_inclusive('\n').enumerate() {
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                consumed += line.len();
                continue;
            }
            // Incomplete last line (no newline yet) — wait for more bytes.
            if !line.ends_with('\n') {
                break;
            }
            let record = serde_json::from_str::<AuditRecord>(trimmed).map_err(|error| {
                self.malformed_record(index + 1, offset + consumed as u64, error)
            })?;
            rows.push(record);
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

    fn remove_audit_file(path: &Path) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(audit_lock_path(path));
    }

    #[test]
    fn append_and_tail() {
        let path = temp_dir().join(format!("omega-audit-{}.jsonl", crate::new_uuid()));
        let log = AuditLog { path: path.clone() };
        log.append(AuditEvent::Step {
            step: 1,
            goal: "g".into(),
            action: "observe".into(),
            avg_err: 0.5,
            damping: 1.0,
        })
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let rows = log.tail(10).unwrap();
        assert_eq!(rows.len(), 1);
        let sum = log.summarize().unwrap();
        assert_eq!(sum.steps, 1);
        assert_eq!(sum.action_counts[0].0, "observe");
        remove_audit_file(&path);
    }

    #[test]
    fn append_limit_accepts_exact_boundary_and_preserves_file_when_full() {
        let event = AuditEvent::ToolAllowed {
            tool: "read".into(),
        };
        let record_len = serde_json::to_vec(&AuditRecord {
            ts: crate::now_utc(),
            event: event.clone(),
        })
        .unwrap()
        .len() as u64
            + 1;
        let path = temp_dir().join(format!("omega-audit-limit-{}.jsonl", crate::new_uuid()));
        let log = AuditLog { path: path.clone() };

        log.append_with_max_bytes(event.clone(), record_len)
            .unwrap();
        assert_eq!(log.byte_len().unwrap(), record_len);
        let before = fs::read(&path).unwrap();

        let error = log
            .append_with_max_bytes(event, record_len)
            .unwrap_err()
            .to_string();
        assert!(error.contains("audit log size limit exceeded"), "{error}");
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(log.read_all().unwrap().len(), 1);
        remove_audit_file(&path);
    }

    #[test]
    fn concurrent_appends_cannot_exceed_size_limit() {
        let event = AuditEvent::ToolAllowed {
            tool: "read".into(),
        };
        let record_len = serde_json::to_vec(&AuditRecord {
            ts: crate::now_utc(),
            event: event.clone(),
        })
        .unwrap()
        .len() as u64
            + 1;
        let path = temp_dir().join(format!("omega-audit-race-{}.jsonl", crate::new_uuid()));
        let log = AuditLog { path: path.clone() };

        let writers: Vec<_> = (0..2)
            .map(|_| {
                let path = path.clone();
                let event = event.clone();
                std::thread::spawn(move || {
                    AuditLog { path }.append_with_max_bytes(event, record_len)
                })
            })
            .collect();
        let outcomes: Vec<_> = writers
            .into_iter()
            .map(|writer| writer.join().unwrap())
            .collect();

        assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(log.read_all().unwrap().len(), 1);
        assert!(log.byte_len().unwrap() <= record_len);
        remove_audit_file(&path);
    }

    #[test]
    fn existing_oversized_audit_files_remain_readable_but_reject_appends() {
        let path = temp_dir().join(format!(
            "omega-audit-legacy-limit-{}.jsonl",
            crate::new_uuid()
        ));
        let log = AuditLog { path: path.clone() };
        let event = AuditEvent::ToolAllowed {
            tool: "read".into(),
        };
        log.append_with_max_bytes(event.clone(), u64::MAX).unwrap();
        log.append_with_max_bytes(event.clone(), u64::MAX).unwrap();
        let existing_len = log.byte_len().unwrap();
        let before = fs::read(&path).unwrap();

        let error = log
            .append_with_max_bytes(event, existing_len - 1)
            .unwrap_err()
            .to_string();
        assert!(error.contains("audit log size limit exceeded"), "{error}");
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(log.read_all().unwrap().len(), 2);
        let (rows, offset) = log.poll_after(0).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(offset, existing_len);
        remove_audit_file(&path);
    }

    #[test]
    fn malformed_records_are_not_silently_dropped() {
        let path = temp_dir().join(format!("omega-corrupt-{}.jsonl", crate::new_uuid()));
        fs::write(&path, b"{not-json}\n").unwrap();
        let log = AuditLog { path: path.clone() };

        let error = log.read_all().unwrap_err().to_string();
        assert!(error.contains("line 1"), "{error}");
        assert!(error.contains("byte offset 0"), "{error}");
        let error = log.poll_after(0).unwrap_err().to_string();
        assert!(error.contains("line 1"), "{error}");
        assert!(log.summarize().is_err());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn poll_after_reports_corruption_with_absolute_offset() {
        let path = temp_dir().join(format!(
            "omega-tail-{}-{}.jsonl",
            std::process::id(),
            crate::new_uuid()
        ));
        let log = AuditLog { path: path.clone() };
        log.append(AuditEvent::Step {
            step: 1,
            goal: "g".into(),
            action: "observe".into(),
            avg_err: 0.5,
            damping: 1.0,
        })
        .unwrap();
        let offset = log.byte_len().unwrap();
        fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"{not-json}\n")
            .unwrap();
        let error = log.poll_after(0).unwrap_err().to_string();
        assert!(error.contains("line 2"), "{error}");
        assert!(error.contains(&format!("byte offset {offset}")), "{error}");

        log.append(AuditEvent::Step {
            step: 2,
            goal: "g".into(),
            action: "explore".into(),
            avg_err: 0.4,
            damping: 1.0,
        })
        .unwrap();
        let complete = fs::read_to_string(&path).unwrap();
        let partial = complete.trim_end().to_string();
        fs::write(&path, partial).unwrap();
        let error = log.poll_after(0).unwrap_err().to_string();
        assert!(error.contains("line 2"), "{error}");
        remove_audit_file(&path);
    }

    #[test]
    fn poll_after_keeps_an_incomplete_trailing_record() {
        let path = temp_dir().join(format!("omega-partial-{}.jsonl", crate::new_uuid()));
        fs::write(&path, b"{\"ts\":").unwrap();
        let log = AuditLog { path: path.clone() };
        let (rows, offset) = log.poll_after(0).unwrap();
        assert!(rows.is_empty());
        assert_eq!(offset, 0);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn poll_after_reads_only_new_lines() {
        let path = temp_dir().join(format!("omega-poll-{}.jsonl", crate::new_uuid()));
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

        remove_audit_file(&path);
    }

    #[test]
    #[cfg(unix)]
    fn appending_does_not_change_permissions_of_an_existing_file() {
        use std::os::unix::fs::PermissionsExt;

        let path = temp_dir().join(format!("omega-audit-existing-{}.jsonl", crate::new_uuid()));
        fs::write(&path, b"").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        AuditLog { path: path.clone() }
            .append(AuditEvent::TaskEpisode {
                task: "existing".into(),
                reward: 0.5,
                steps: 1,
            })
            .unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
        remove_audit_file(&path);
    }
}
