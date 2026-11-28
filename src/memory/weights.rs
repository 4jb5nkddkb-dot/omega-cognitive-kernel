#[allow(unused_imports)]
use crate::prelude::*;

use alloc::collections::BTreeMap;
use serde::{Deserialize, Serialize};

use crate::error::{OmegaError, Result};
use crate::safety::crypto::{hex_encode, sha256};

pub const WEIGHTS_MAGIC: &str = "OMEGAWTS";
pub const WEIGHTS_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeightsPayload {
    pub magic: String,
    pub version: u32,
    pub weights: BTreeMap<String, f64>,
    pub delta_history: Vec<BTreeMap<String, f64>>,
    pub extra: BTreeMap<String, serde_json::Value>,
    pub checksum: String,
}

fn compute_checksum(
    magic: &str,
    version: u32,
    weights: &BTreeMap<String, f64>,
    delta_history: &[BTreeMap<String, f64>],
) -> Result<String> {
    let body = serde_json::json!({
        "magic": magic,
        "version": version,
        "weights": weights,
        "delta_history": delta_history,
    });
    let bytes = serde_json::to_vec(&body).map_err(|e| OmegaError::Persist(e.to_string()))?;
    Ok(hex_encode(&sha256(&bytes)))
}

pub fn export_weights(
    weights: BTreeMap<String, f64>,
    delta_history: Vec<BTreeMap<String, f64>>,
    extra: BTreeMap<String, serde_json::Value>,
) -> Result<WeightsPayload> {
    let checksum = compute_checksum(WEIGHTS_MAGIC, WEIGHTS_VERSION, &weights, &delta_history)?;
    Ok(WeightsPayload {
        magic: WEIGHTS_MAGIC.into(),
        version: WEIGHTS_VERSION,
        weights,
        delta_history,
        extra,
        checksum,
    })
}

pub fn verify_weights(payload: &WeightsPayload) -> bool {
    if payload.magic != WEIGHTS_MAGIC || payload.version == 0 || payload.version > WEIGHTS_VERSION {
        return false;
    }
    if payload.weights.values().any(|v| !v.is_finite()) {
        return false;
    }
    let Ok(expected) = compute_checksum(
        &payload.magic,
        payload.version,
        &payload.weights,
        &payload.delta_history,
    ) else {
        return false;
    };
    payload.checksum == expected
}

pub fn migrate_payload(payload: &WeightsPayload) -> Result<WeightsPayload> {
    if payload.magic != WEIGHTS_MAGIC || payload.version == 0 {
        return Err(OmegaError::Persist("invalid weights payload header".into()));
    }
    let mut clean_weights = BTreeMap::new();
    for (k, &v) in &payload.weights {
        clean_weights.insert(k.clone(), if v.is_finite() { v } else { 0.0 });
    }
    let clean_deltas = payload.delta_history.clone();
    let mut extra = payload.extra.clone();
    if payload.version < WEIGHTS_VERSION {
        extra.insert("migrated_from".into(), serde_json::json!(payload.version));
    }
    export_weights(clean_weights, clean_deltas, extra)
}

pub fn squash_delta_history(payload: &mut WeightsPayload, keep_recent: usize) -> Result<usize> {
    if payload.delta_history.len() <= keep_recent {
        return Ok(0);
    }
    let split = payload.delta_history.len() - keep_recent;
    let mut weights = payload.weights.clone();
    for delta in &payload.delta_history[..split] {
        for (k, v) in delta {
            if v.is_finite() {
                let updated = weights.get(k).copied().unwrap_or(0.0) + v;
                if !updated.is_finite() {
                    return Err(OmegaError::Persist(
                        "squashing weight history would produce a non-finite weight".into(),
                    ));
                }
                weights.insert(k.clone(), updated);
            }
        }
    }

    let remaining = payload.delta_history[split..].to_vec();
    let checksum = compute_checksum(&payload.magic, payload.version, &weights, &remaining)?;
    payload.weights = weights;
    payload.delta_history = remaining;
    payload.checksum = checksum;
    Ok(split)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_verify_migrate_and_squash_roundtrip() {
        let mut weights = BTreeMap::new();
        weights.insert("w0".into(), 1.0);
        let mut d1 = BTreeMap::new();
        d1.insert("w0".into(), 0.25);
        let mut d2 = BTreeMap::new();
        d2.insert("w0".into(), 0.5);

        let mut payload = export_weights(weights, vec![d1, d2], BTreeMap::new()).expect("export");
        assert!(verify_weights(&payload));

        let mut tampered = payload.clone();
        tampered.weights.insert("w0".into(), 99.0);
        assert!(!verify_weights(&tampered));

        let squashed = squash_delta_history(&mut payload, 1).expect("squash");
        assert_eq!(squashed, 1);
        assert!(verify_weights(&payload));
        assert!((payload.weights["w0"] - 1.25).abs() < 1e-9);
        assert_eq!(payload.delta_history.len(), 1);

        let mut legacy = payload.clone();
        legacy.version = 1;
        let upgraded = migrate_payload(&legacy).expect("migrate");
        assert_eq!(upgraded.version, WEIGHTS_VERSION);
        assert!(verify_weights(&upgraded));
    }

    #[test]
    fn squash_delta_history_rejects_overflow_atomically() {
        for delta_value in [f64::MAX, -f64::MAX] {
            let mut weights = BTreeMap::new();
            weights.insert("w".into(), 0.0);
            let mut deltas = Vec::new();
            for _ in 0..2 {
                let mut delta = BTreeMap::new();
                delta.insert("w".into(), delta_value);
                deltas.push(delta);
            }

            let mut payload = export_weights(weights, deltas, BTreeMap::new()).expect("export");
            let original = payload.clone();

            let result = squash_delta_history(&mut payload, 0);
            assert!(result.is_err(), "overflow should be rejected");
            assert_eq!(payload, original, "rejection must preserve the payload");
            assert!(verify_weights(&payload));
        }
    }
}
