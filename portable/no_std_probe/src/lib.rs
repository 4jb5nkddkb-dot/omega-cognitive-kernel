#![no_std]

extern crate alloc;

use alloc::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};

/// Minimal embedded-side state envelope. This deliberately has no filesystem,
/// networking, allocator-global, or OS dependency. It is the contract the
/// future kernel-core crate can expose to embedded hosts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmbeddedSnapshot {
    pub schema_version: u16,
    pub step: u64,
    pub fixed_point: bool,
    pub dwell_steps: u32,
    pub score: f32,
    pub active_module: Option<String>,
    pub detector_history: Vec<f32>,
}

impl EmbeddedSnapshot {
    pub const fn new(step: u64) -> Self {
        Self {
            schema_version: 1,
            step,
            fixed_point: false,
            dwell_steps: 0,
            score: 0.0,
            active_module: None,
            detector_history: Vec::new(),
        }
    }
}
