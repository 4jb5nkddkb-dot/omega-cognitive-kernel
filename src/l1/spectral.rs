#[allow(unused_imports)]
use crate::prelude::*;

use core::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpectralBin {
    pub re: f64,
    pub im: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpectralPatch {
    pub dc_baseline: f64,
    pub high_freq_ratio: f64,
    pub filtered_spectrum: Vec<SpectralBin>,
}

#[derive(Debug, Clone)]
pub struct L1SpectralReflector {
    pub window_size: usize,
    pub energy_threshold: f64,
    pub low_freq_cutoff: usize,
}

impl Default for L1SpectralReflector {
    fn default() -> Self {
        Self {
            window_size: 16,
            energy_threshold: 1e-3,
            low_freq_cutoff: 2,
        }
    }
}

impl L1SpectralReflector {
    pub fn new(window_size: usize, energy_threshold: f64, low_freq_cutoff: usize) -> Self {
        Self {
            window_size: window_size.max(2),
            energy_threshold: energy_threshold.max(0.0),
            low_freq_cutoff: low_freq_cutoff.max(1),
        }
    }

    pub fn reflect_and_patch(&self, l0_rewards: &[f64]) -> Option<SpectralPatch> {
        let n = self.window_size;
        if n < 2 || l0_rewards.len() < n {
            return None;
        }
        let win = &l0_rewards[l0_rewards.len() - n..];
        let nf = n as f64;
        let dc_baseline = win.iter().sum::<f64>() / nf;

        let mut spectrum = Vec::with_capacity(n);
        for k in 0..n {
            let mut re = 0.0;
            let mut im = 0.0;
            for (idx, &x) in win.iter().enumerate() {
                let angle = 2.0 * PI * (k as f64) * (idx as f64) / nf;
                re += x * libm::cos(angle);
                im -= x * libm::sin(angle);
            }
            spectrum.push(SpectralBin { re, im });
        }

        let half = n / 2;
        let mut total_ac = 0.0;
        let mut high_ac = 0.0;
        for (k, bin) in spectrum.iter().enumerate().skip(1).take(half) {
            let weight = if k * 2 == n { 1.0 } else { 2.0 };
            let mag2 = (bin.re * bin.re + bin.im * bin.im) * weight / (nf * nf);
            total_ac += mag2;
            if k > self.low_freq_cutoff {
                high_ac += mag2;
            }
        }

        if total_ac <= self.energy_threshold {
            return None;
        }

        let high_freq_ratio = (high_ac / total_ac).clamp(0.0, 1.0);
        let cutoff = self.low_freq_cutoff.min(half);
        let mut filtered_spectrum = Vec::with_capacity(n);
        for (k, bin) in spectrum.iter().enumerate() {
            let keep = k == 0 || k <= cutoff || (n - k) <= cutoff;
            if keep {
                filtered_spectrum.push(*bin);
            } else {
                filtered_spectrum.push(SpectralBin { re: 0.0, im: 0.0 });
            }
        }

        Some(SpectralPatch {
            dc_baseline,
            high_freq_ratio,
            filtered_spectrum,
        })
    }

    pub fn inverse_restore(filtered_spectrum: &[SpectralBin]) -> Vec<f64> {
        let n = filtered_spectrum.len();
        if n == 0 {
            return Vec::new();
        }
        let nf = n as f64;
        let mut out = Vec::with_capacity(n);
        for idx in 0..n {
            let mut sum = 0.0;
            for (k, bin) in filtered_spectrum.iter().enumerate() {
                let angle = 2.0 * PI * (k as f64) * (idx as f64) / nf;
                sum += bin.re * libm::cos(angle) - bin.im * libm::sin(angle);
            }
            out.push(sum / nf);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_signal_stays_silent_and_step_signal_restores() {
        let reflector = L1SpectralReflector::new(8, 1e-4, 1);
        let flat = [0.5; 8];
        assert!(reflector.reflect_and_patch(&flat).is_none());

        let step = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0];
        let patch = reflector.reflect_and_patch(&step).expect("step wakes L1");
        assert!(patch.high_freq_ratio < 0.35);
        let restored = L1SpectralReflector::inverse_restore(&patch.filtered_spectrum);
        assert_eq!(restored.len(), 8);
        assert!(restored[7] > restored[0]);
    }
}
