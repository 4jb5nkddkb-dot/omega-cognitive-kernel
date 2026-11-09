//! # Inbox vertical demo (product story)
//!
//! Simulates an external system pushing **text / event** jobs into a queue.
//! The kernel **observes** the queue, **decides** under a bounded policy
//! (`RuleEffect`), and **executes** through a pluggable gate that only logs
//! (safe for demos). Audit + action log land on disk.
//!
//! ## What this shows (honest scope)
//!
//! - **Pluggable I/O**: swap `Observer` / `Executor` without changing the step loop
//! - **Online policy patches**: under rising load/error, L1 may commit a `RuleEffect`
//! - **Audit trail**: kernel `audit.jsonl` + side-channel `actions.log`
//!
//! This is **not** an NLP content-moderation filter. Filtering here means
//! *policy preference* (explore / exploit / stabilize) under constraints —
//! the same product surface as production integrators use.
//!
//! ```bash
//! cargo run --example inbox_vertical
//! # keep workspace on disk:
//! KEEP_INBOX=1 cargo run --example inbox_vertical
//! ```

use chrono::Utc;
use omega_cognitive_kernel::types::{Action, Modality};
use omega_cognitive_kernel::Executor;
use omega_cognitive_kernel::{Kernel, KernelConfig, Observation, Observer, Result};
use serde_json::json;
use std::collections::{HashSet, VecDeque};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// ─── Plug-in: event queue observer ───────────────────────────────────────────

/// Shared inbox of text events (what an external producer would push).
#[derive(Clone, Default)]
struct EventInbox {
    inner: Arc<Mutex<VecDeque<InboxEvent>>>,
}

#[derive(Clone, Debug)]
struct InboxEvent {
    id: String,
    text: String,
    priority: u8,
}

impl EventInbox {
    fn push(&self, id: impl Into<String>, text: impl Into<String>, priority: u8) {
        self.inner.lock().unwrap().push_back(InboxEvent {
            id: id.into(),
            text: text.into(),
            priority,
        });
    }

    fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    fn snapshot(&self) -> Vec<InboxEvent> {
        self.inner.lock().unwrap().iter().cloned().collect()
    }
}

/// Observer adapter: each step reports queue depth + recent event heads.
struct EventQueueObserver {
    inbox: EventInbox,
    step: u64,
    max_heads: usize,
}

impl EventQueueObserver {
    fn new(inbox: EventInbox) -> Self {
        Self {
            inbox,
            step: 0,
            max_heads: 8,
        }
    }
}

impl Observer for EventQueueObserver {
    fn observe(&mut self) -> Result<Vec<Observation>> {
        self.step += 1;
        let events = self.inbox.snapshot();
        let depth = events.len();
        let heads: Vec<_> = events
            .iter()
            .take(self.max_heads)
            .map(|e| {
                json!({
                    "id": e.id,
                    "priority": e.priority,
                    "text_preview": truncate(&e.text, 48),
                })
            })
            .collect();

        // Higher depth → higher synthetic "pressure" signal for the predictor loop.
        let pressure = (depth as f64 / 10.0).min(1.0);

        Ok(vec![Observation {
            timestamp: Utc::now(),
            modality: Modality::Structured,
            content: json!({
                "step": self.step,
                "queue_depth": depth,
                "pressure": pressure,
                "heads": heads,
            }),
            confidence: 0.95,
            source: "inbox:event_queue".into(),
        }])
    }
}

// ─── Plug-in: policy gate executor (log-only) ────────────────────────────────

/// Executor adapter: whitelist + structured decision log (no dangerous side effects).
struct PolicyGateExecutor {
    allowed: HashSet<String>,
    log_path: PathBuf,
    echo: bool,
}

impl PolicyGateExecutor {
    fn new(allowed: impl IntoIterator<Item = String>, log_path: impl AsRef<Path>) -> Self {
        Self {
            allowed: allowed.into_iter().collect(),
            log_path: log_path.as_ref().to_path_buf(),
            echo: true,
        }
    }
}

impl Executor for PolicyGateExecutor {
    fn execute(
        &mut self,
        actions: &[Action],
    ) -> omega_cognitive_kernel::Result<Vec<serde_json::Value>> {
        let mut out = Vec::new();
        for act in actions {
            if !act.constraints_checked {
                return Err(omega_cognitive_kernel::OmegaError::ExecutionBlocked(
                    format!("{} (constraints not checked)", act.name),
                ));
            }
            if !self.allowed.contains(&act.name) {
                return Err(omega_cognitive_kernel::OmegaError::ExecutionBlocked(
                    act.name.clone(),
                ));
            }

            // Product framing: map action name → handling posture for operators.
            let posture = match act.name.as_str() {
                "stabilize" => "hold_and_drain",
                "observe" => "inspect_only",
                "explore" => "sample_new_strategy",
                "exploit" => "apply_known_strategy",
                _ => "noop",
            };

            let entry = json!({
                "ts": Utc::now().to_rfc3339(),
                "action": act.name,
                "posture": posture,
                "params": act.params,
                "gate": "allow",
            });

            if self.echo {
                println!("  [gate] {} → posture={posture}", act.name);
            }

            let mut f = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.log_path)
                .map_err(|e| {
                    omega_cognitive_kernel::OmegaError::ExecutionBlocked(format!("log: {e}"))
                })?;
            writeln!(f, "{entry}").map_err(|e| {
                omega_cognitive_kernel::OmegaError::ExecutionBlocked(format!("log write: {e}"))
            })?;

            out.push(entry);
        }
        Ok(out)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max).collect();
        format!("{t}…")
    }
}

fn whitelist() -> Vec<String> {
    vec![
        "noop".into(),
        "observe".into(),
        "explore".into(),
        "exploit".into(),
        "stabilize".into(),
    ]
}

// ─── Demo driver ─────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let keep = env::var("KEEP_INBOX").ok().as_deref() == Some("1");
    let root = if keep {
        PathBuf::from("./omega-inbox-demo")
    } else {
        env::temp_dir().join(format!("omega-inbox-demo-{}", uuid::Uuid::new_v4()))
    };
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("mkdir");

    let state_path = root.join("state.json");
    let cfg_path = root.join("omega.toml");
    let actions_path = root.join("actions.log");

    // Tight cooldown so the demo is likely to show at least one patch under load.
    let cfg = KernelConfig {
        state_path: Some(state_path.display().to_string()),
        autosave: true,
        auto_checkpoint_on_patch: true,
        error_threshold: 0.25,
        min_steps_between_patches: 2,
        max_risk: 0.50,
        default_goal: Some(
            "drain inbox under rising load; prefer stabilize when pressure high".into(),
        ),
        ..KernelConfig::default()
    };
    cfg.save(&cfg_path)?;

    println!("╔══════════════════════════════════════════════════╗");
    println!("║  Omega Inbox Vertical — pluggable observe/act    ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!("workspace : {}", root.display());
    println!("config    : {}", cfg_path.display());
    println!("state     : {}", state_path.display());
    println!("actions   : {}", actions_path.display());
    println!("audit     : beside state (audit.jsonl)");
    println!();

    let inbox = EventInbox::default();

    // ── Mount plugins (this is the whole integration surface) ───────────────
    let observer = EventQueueObserver::new(inbox.clone());
    let executor = PolicyGateExecutor::new(whitelist(), &actions_path);
    let mut kernel = Kernel::with_adapters_and_path(observer, executor, &state_path);
    kernel.apply_config(&KernelConfig::load(&cfg_path)?);

    let goal = cfg.default_goal.clone().unwrap();

    // Scripted "external producer" stream of text/events.
    let script: &[(&str, &str, u8)] = &[
        ("e1", "user signup from region=us-east", 1),
        ("e2", "batch job: reindex catalog", 2),
        ("e3", "ALERT: queue lag 2.1s", 5),
        ("e4", "ALERT: error rate spike 4%", 5),
        ("e5", "deploy canary 10%", 3),
        ("e6", "ALERT: disk pressure 92%", 5),
        ("e7", "customer ticket #4412 escalated", 4),
        ("e8", "nightly backup complete", 1),
        ("e9", "ALERT: p99 latency 1.8s", 5),
        ("e10", "scale-out request from autoscaler", 3),
    ];

    println!("── live stream ──");
    for (i, (id, text, pri)) in script.iter().enumerate() {
        let step = (i + 1) as u32;
        inbox.push(*id, *text, *pri);
        println!(
            "[world] +{id} pri={pri} depth={}  «{}»",
            inbox.len(),
            truncate(text, 40)
        );

        let r = kernel.step(&goal)?;
        let act = r
            .trajectory
            .actions
            .first()
            .map(|a| a.name.as_str())
            .unwrap_or("-");
        let st = kernel.status();
        print!(
            "[kernel] step={step:<2} act={act:<9} patched={:<5} err={:.3} damp={:.2}",
            r.l1.patched, st.avg_prediction_error, st.error_damping
        );
        if r.l1.patched {
            print!("  ★ RuleEffect committed");
        }
        println!();
    }

    kernel.save()?;

    // ── Evidence operators care about ───────────────────────────────────────
    println!("\n── status ──");
    let st = kernel.status();
    println!(
        "steps={}  avg_err={:.3}  damp={:.3}  explore={:.3}  risk_tol={:.3}  max_risk={:.2}",
        st.total_steps,
        st.avg_prediction_error,
        st.error_damping,
        st.explore_bonus,
        st.risk_tolerance,
        st.max_risk
    );
    println!(
        "modules={}  checkpoints={}  L2 in_fp={} align={:.3}",
        st.active_modules, st.checkpoint_count, st.in_fixed_point, st.last_alignment_score
    );

    println!("\n── audit.jsonl (tail) ──");
    match kernel.audit_tail(12) {
        Ok(rows) if rows.is_empty() => println!("(empty)"),
        Ok(rows) => {
            for row in rows {
                println!("{}  {:?}", row.ts.to_rfc3339(), row.event);
            }
        }
        Err(e) => println!("(audit read: {e})"),
    }

    if actions_path.exists() {
        let body = fs::read_to_string(&actions_path).unwrap_or_default();
        println!("\n── actions.log ({} lines) ──", body.lines().count());
        for line in body
            .lines()
            .rev()
            .take(5)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            println!("{line}");
        }
    }

    println!("\n── takeaway ──");
    println!("1. EventInbox + EventQueueObserver  = observe plugin");
    println!("2. PolicyGateExecutor              = execute plugin (log-only gate)");
    println!("3. Kernel::with_adapters           = core loop unchanged");
    println!("4. audit.jsonl + actions.log       = operator evidence");
    if keep {
        println!("\nKEEP_INBOX=1 → workspace kept at {}", root.display());
    } else {
        let _ = fs::remove_dir_all(&root);
        println!("\n(temp workspace removed; set KEEP_INBOX=1 to inspect files)");
    }
    Ok(())
}
