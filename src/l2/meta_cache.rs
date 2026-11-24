#[allow(unused_imports)]
use crate::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ContextFingerprint {
    pub error_mean: f64,
    pub error_variance: f64,
    pub rollback_rate: f64,
    pub action_entropy: f64,
}
impl ContextFingerprint {
    /// Entropy is supplied by L0's actual two-arm KL policy, not inferred from
    /// rewards (a reward is not an action label).
    pub fn from_window(errors: &[f64], rollbacks: u32, total: u32, action_entropy: f64) -> Self {
        let n = errors.len().max(1) as f64;
        let mean = errors.iter().sum::<f64>() / n;
        let variance = errors.iter().map(|e| (e - mean) * (e - mean)).sum::<f64>() / n;
        Self {
            error_mean: mean,
            error_variance: variance,
            rollback_rate: rollbacks as f64 / total.max(1) as f64,
            action_entropy,
        }
    }
    pub fn similarity(&self, other: &Self) -> f64 {
        let distance = (self.error_mean - other.error_mean).abs()
            + libm::sqrt((self.error_variance - other.error_variance).abs())
            + (self.rollback_rate - other.rollback_rate).abs()
            + 0.5 * (self.action_entropy - other.action_entropy).abs();
        (1.0 - distance / 2.5).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaCacheEntry {
    pub fingerprint: ContextFingerprint,
    pub expert_weights: [f64; 4],
    pub alpha: f64,
    pub credit: f64,
    pub last_used: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MetaCache {
    pub entries: Vec<MetaCacheEntry>,
    pub capacity: usize,
    pub hit_threshold: f64,
    pub clock: u64,
    pub last_hit: bool,
}
impl Default for MetaCache {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            capacity: 16,
            hit_threshold: 0.78,
            clock: 0,
            last_hit: false,
        }
    }
}
impl MetaCache {
    pub fn store(
        &mut self,
        fingerprint: ContextFingerprint,
        weights: [f64; 4],
        alpha: f64,
        credit: f64,
    ) {
        self.clock = self.clock.saturating_add(1);
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|e| e.fingerprint.similarity(&fingerprint) > 0.96)
        {
            entry.expert_weights = weights;
            entry.alpha = alpha;
            entry.credit = credit.max(entry.credit);
            entry.last_used = self.clock;
        } else {
            self.entries.push(MetaCacheEntry {
                fingerprint,
                expert_weights: weights,
                alpha,
                credit,
                last_used: self.clock,
            });
        }
        if self.entries.len() > self.capacity.max(1) {
            let index = self
                .entries
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    (a.credit + a.last_used as f64 * 1.0e-6)
                        .partial_cmp(&(b.credit + b.last_used as f64 * 1.0e-6))
                        .unwrap_or(core::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.entries.remove(index);
        }
    }
    pub fn lookup(&mut self, fingerprint: &ContextFingerprint) -> Option<(usize, [f64; 4], f64)> {
        self.last_hit = false;
        let best = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(i, entry)| {
                let similarity = entry.fingerprint.similarity(fingerprint);
                (similarity >= self.hit_threshold).then_some((i, similarity))
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));
        if let Some((index, _)) = best {
            self.clock = self.clock.saturating_add(1);
            self.entries[index].last_used = self.clock;
            self.entries[index].credit += 1.0;
            self.last_hit = true;
            return Some((
                index,
                self.entries[index].expert_weights,
                self.entries[index].alpha,
            ));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_context_hits_and_cache_is_bounded() {
        let mut cache = MetaCache {
            capacity: 2,
            ..MetaCache::default()
        };
        let fingerprint = ContextFingerprint {
            error_mean: 0.4,
            error_variance: 0.02,
            rollback_rate: 0.1,
            action_entropy: 0.7,
        };
        cache.store(fingerprint, [0.1, 0.8, 0.05, 0.05], 0.24, 1.0);
        let (_, weights, alpha) = cache.lookup(&fingerprint).expect("warm start");
        assert_eq!(weights[1], 0.8);
        assert_eq!(alpha, 0.24);
        assert!(cache.last_hit);
        for error_mean in [0.6, 0.8, 0.95] {
            cache.store(
                ContextFingerprint {
                    error_mean,
                    ..fingerprint
                },
                [0.25; 4],
                0.1,
                0.0,
            );
        }
        assert!(cache.entries.len() <= 2);
        let encoded = serde_json::to_string(&cache).unwrap();
        let decoded: MetaCache = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.entries.len(), cache.entries.len());
    }
}
