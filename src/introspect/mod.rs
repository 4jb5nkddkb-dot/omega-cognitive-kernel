#[allow(unused_imports)]
use crate::prelude::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntropyReport {
    pub shannon_entropy: f64,
    pub dead_loop_detected: bool,
    pub strong_attractor_detected: bool,
    pub write_authorized: bool,
}

#[derive(Debug, Clone)]
pub struct EntropyWatchdog {
    pub dead_loop_threshold: f64,
    pub strong_attractor_threshold: f64,
}

impl Default for EntropyWatchdog {
    fn default() -> Self {
        Self {
            dead_loop_threshold: 0.15,
            strong_attractor_threshold: 0.35,
        }
    }
}

impl EntropyWatchdog {
    pub fn inspect(&self, probs: &[f64]) -> EntropyReport {
        let sum: f64 = probs
            .iter()
            .copied()
            .filter(|p| p.is_finite() && *p > 0.0)
            .sum();
        let mut entropy = 0.0;
        if sum > 0.0 {
            for &p in probs {
                if p.is_finite() && p > 0.0 {
                    let norm = p / sum;
                    entropy -= norm * libm::log2(norm);
                }
            }
        }
        EntropyReport {
            shannon_entropy: entropy,
            dead_loop_detected: entropy <= self.dead_loop_threshold,
            strong_attractor_detected: entropy <= self.strong_attractor_threshold,
            write_authorized: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watchdog_detects_collapsed_distribution_and_never_authorizes_writes() {
        let wd = EntropyWatchdog::default();
        let collapsed = wd.inspect(&[0.99, 0.01]);
        assert!(collapsed.dead_loop_detected);
        assert!(!collapsed.write_authorized);

        let uniform = wd.inspect(&[0.5, 0.5]);
        assert!(!uniform.dead_loop_detected);
        assert!((uniform.shannon_entropy - 1.0).abs() < 1e-9);
    }
}
