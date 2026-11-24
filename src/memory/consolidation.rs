#[allow(unused_imports)]
use crate::prelude::*;

use alloc::collections::BTreeMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsolidatedTrace {
    pub episodes_merged: usize,
    pub mean_error: f64,
    pub centroid: BTreeMap<String, f64>,
    pub write_authorized: bool,
}

pub fn obs_distance(a: &BTreeMap<String, f64>, b: &BTreeMap<String, f64>) -> f64 {
    let mut keys = alloc::collections::BTreeSet::new();
    for k in a.keys() {
        keys.insert(k.as_str());
    }
    for k in b.keys() {
        keys.insert(k.as_str());
    }
    if keys.is_empty() {
        return 0.0;
    }
    let mut diff = 0.0;
    for k in &keys {
        diff += (a.get(*k).copied().unwrap_or(0.0) - b.get(*k).copied().unwrap_or(0.0)).abs();
    }
    diff / keys.len() as f64
}

pub fn consolidate_episodes(
    observations: &[BTreeMap<String, f64>],
    errors: &[f64],
) -> ConsolidatedTrace {
    let n = observations.len();
    let mut sums: BTreeMap<String, f64> = BTreeMap::new();
    for obs in observations {
        for (k, v) in obs {
            *sums.entry(k.clone()).or_insert(0.0) += *v;
        }
    }
    let centroid = if n > 0 {
        sums.into_iter().map(|(k, v)| (k, v / n as f64)).collect()
    } else {
        BTreeMap::new()
    };
    let mean_error = if errors.is_empty() {
        0.0
    } else {
        errors.iter().sum::<f64>() / errors.len() as f64
    };
    ConsolidatedTrace {
        episodes_merged: n,
        mean_error,
        centroid,
        write_authorized: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consolidation_computes_centroid_and_distance() {
        let mut a = BTreeMap::new();
        a.insert("x".into(), 0.0);
        let mut b = BTreeMap::new();
        b.insert("x".into(), 1.0);
        assert!((obs_distance(&a, &b) - 1.0).abs() < 1e-9);

        let trace = consolidate_episodes(&[a, b], &[0.2, 0.4]);
        assert_eq!(trace.episodes_merged, 2);
        assert!((trace.centroid["x"] - 0.5).abs() < 1e-9);
        assert!(!trace.write_authorized);
    }
}
