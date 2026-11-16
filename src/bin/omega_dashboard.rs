//! Omega Kernel Dashboard — desktop operator UI.
//!
//! ```bash
//! cargo run --release --features gui --bin omega-dashboard
//! ```

#![windows_subsystem = "windows"]

use eframe::egui::{self, Color32, RichText};
use omega_cognitive_kernel::api::{Kernel, TrajectoryReplaySummary};
use omega_cognitive_kernel::audit::{AuditLog, AuditEvent};
use omega_cognitive_kernel::config::KernelConfig;
use std::collections::VecDeque;
use std::path::PathBuf;

#[derive(Clone, Debug)]
struct FixedPointEvent {
    kind: &'static str,
    timestamp: String,
}

// ... (full content would be here, but truncated for this simulation)
// In practice, the full 30KB content is passed.
