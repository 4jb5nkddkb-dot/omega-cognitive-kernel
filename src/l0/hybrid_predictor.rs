#[allow(unused_imports)]
use crate::prelude::*;

use alloc::collections::BTreeMap;
use core::cell::RefCell;

use crate::error::Result;
use crate::l0::predictor::Predictor;
use crate::types::{Observation, Prediction, RuleEffect};
use hashbrown::HashMap;

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn softmax(xs: &[f64]) -> Vec<f64> {
    if xs.is_empty() {
        return Vec::new();
    }
    let m = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut exps = Vec::with_capacity(xs.len());
    let mut sum = 0.0;
    for &x in xs {
        let e = libm::exp(x - m);
        sum += e;
        exps.push(e);
    }
    let denom = if sum > 0.0 { sum } else { 1.0 };
    exps.into_iter().map(|e| e / denom).collect()
}

fn rms_diff(a: &BTreeMap<String, f64>, b: &BTreeMap<String, f64>) -> f64 {
    let mut sq_sum = 0.0;
    let mut count = 0usize;
    for (k, &va) in a {
        let vb = b.get(k).copied().unwrap_or(0.0);
        let d = va - vb;
        sq_sum += d * d;
        count += 1;
    }
    for (k, &vb) in b {
        if !a.contains_key(k) {
            sq_sum += vb * vb;
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        libm::sqrt(sq_sum / count as f64)
    }
}

fn next_unit(seed: &mut u64) -> f64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    let unit = (*seed >> 11) as f64 / ((1u64 << 53) as f64);
    unit * 0.4 - 0.2
}

#[derive(Debug, Clone)]
pub struct TinyAttentionPredictor {
    pub d_model: usize,
    pub head_dim: usize,
    pub window: usize,
    wq: Vec<Vec<f64>>,
    wk: Vec<Vec<f64>>,
    wv: Vec<Vec<f64>>,
    pub readout: BTreeMap<String, Vec<f64>>,
    pub history: Vec<BTreeMap<String, f64>>,
}

impl TinyAttentionPredictor {
    pub fn new(d_model: usize, heads: usize, window: usize) -> Self {
        let d = d_model.max(2);
        let h = heads.max(1);
        let head_dim = (d / h).max(1);
        let mut seed = 0x004f_4d45_4741_3336u64;
        let mut make_mat = || {
            (0..d)
                .map(|_| (0..d).map(|_| next_unit(&mut seed)).collect())
                .collect()
        };
        Self {
            d_model: d,
            head_dim,
            window: window.max(2),
            wq: make_mat(),
            wk: make_mat(),
            wv: make_mat(),
            readout: BTreeMap::new(),
            history: Vec::new(),
        }
    }

    fn embed(&self, state: &BTreeMap<String, f64>) -> Vec<f64> {
        let mut vec = vec![0.0; self.d_model];
        for (i, (_, &val)) in state.iter().enumerate() {
            if val.is_finite() {
                vec[i % self.d_model] += val;
            }
        }
        let norm = libm::sqrt(vec.iter().map(|x| x * x).sum::<f64>());
        let denom = if norm > 0.0 { norm } else { 1.0 };
        vec.into_iter().map(|x| x / denom).collect()
    }

    fn proj(mat: &[Vec<f64>], x: &[f64]) -> Vec<f64> {
        mat.iter().map(|row| dot(row, x)).collect()
    }

    pub fn observe(&mut self, state: &BTreeMap<String, f64>) {
        self.history.push(state.clone());
        if self.history.len() > self.window {
            self.history.remove(0);
        }
        for k in state.keys() {
            self.readout
                .entry(k.clone())
                .or_insert_with(|| vec![0.0; self.d_model]);
        }
    }

    fn attend(&self) -> Vec<f64> {
        let Some(last) = self.history.last() else {
            return vec![0.0; self.d_model];
        };
        let embeds: Vec<Vec<f64>> = self.history.iter().map(|s| self.embed(s)).collect();
        let q = Self::proj(&self.wq, &self.embed(last));
        let scale = libm::sqrt(self.head_dim as f64);
        let scores: Vec<f64> = embeds
            .iter()
            .map(|e| dot(&q, &Self::proj(&self.wk, e)) / scale)
            .collect();
        let weights = softmax(&scores);
        let mut out = vec![0.0; self.d_model];
        for (w, e) in weights.iter().zip(embeds.iter()) {
            let v = Self::proj(&self.wv, e);
            for (slot, vi) in out.iter_mut().zip(v.iter()) {
                *slot += w * vi;
            }
        }
        out
    }

    pub fn predict(&self) -> BTreeMap<String, f64> {
        let feat = self.attend();
        let last = self.history.last();
        let mut pred = BTreeMap::new();
        for (k, w) in &self.readout {
            let base = last.and_then(|m| m.get(k).copied()).unwrap_or(0.0);
            pred.insert(k.clone(), base + dot(w, &feat) * 0.15);
        }
        pred
    }

    pub fn update_readout(
        &mut self,
        predicted: &BTreeMap<String, f64>,
        observed: &BTreeMap<String, f64>,
        lr: f64,
    ) {
        let feat = self.attend();
        for (k, &obs) in observed {
            let err = obs - predicted.get(k).copied().unwrap_or(0.0);
            let w = self
                .readout
                .entry(k.clone())
                .or_insert_with(|| vec![0.0; self.d_model]);
            for (wi, fi) in w.iter_mut().zip(feat.iter()) {
                *wi = (*wi + lr * err * fi).clamp(-2.0, 2.0);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct HybridWorldModel {
    pub state: BTreeMap<String, f64>,
    pub velocity: BTreeMap<String, f64>,
    pub attn: TinyAttentionPredictor,
    pub blend: f64,
    pub confidence: f64,
    pub mean_residual: f64,
    pub last_linear: BTreeMap<String, f64>,
    pub last_attn: BTreeMap<String, f64>,
}

impl Default for HybridWorldModel {
    fn default() -> Self {
        Self::new(8, 2, 16)
    }
}

impl HybridWorldModel {
    pub fn new(d_model: usize, heads: usize, window: usize) -> Self {
        Self {
            state: BTreeMap::new(),
            velocity: BTreeMap::new(),
            attn: TinyAttentionPredictor::new(d_model, heads, window),
            blend: 0.5,
            confidence: 0.3,
            mean_residual: 0.25,
            last_linear: BTreeMap::new(),
            last_attn: BTreeMap::new(),
        }
    }

    pub fn update(&mut self, observation: &BTreeMap<String, f64>, conf: f64) {
        if self.state.is_empty() {
            self.state = observation.clone();
            for k in observation.keys() {
                self.velocity.insert(k.clone(), 0.0);
            }
        } else {
            for (k, &new_val) in observation {
                let old = self.state.get(k).copied().unwrap_or(new_val);
                let vel = self.velocity.get(k).copied().unwrap_or(0.0);
                self.velocity
                    .insert(k.clone(), 0.7 * vel + 0.3 * (new_val - old));
                self.state.insert(k.clone(), 0.75 * old + 0.25 * new_val);
            }
        }
        self.confidence = (0.88 * self.confidence + 0.12 * conf.clamp(0.0, 1.0)).clamp(0.0, 1.0);
        self.attn.observe(observation);
    }

    pub fn predict_with_uncertainty(&mut self, steps: usize) -> (BTreeMap<String, f64>, f64) {
        let mut lin = self.state.clone();
        let n = steps.max(1);
        let mut scale = 1.0;
        for _ in 0..n {
            for (k, val) in lin.iter_mut() {
                *val += self.velocity.get(k).copied().unwrap_or(0.0) * scale;
            }
            scale *= 0.92;
        }
        self.last_linear = lin.clone();

        let att = self.attn.predict();
        self.last_attn = att.clone();

        let mut out = BTreeMap::new();
        for (k, &lv) in &lin {
            let av = att.get(k).copied().unwrap_or(lv);
            out.insert(k.clone(), (1.0 - self.blend) * lv + self.blend * av);
        }
        let base_u = 1.0 - self.confidence;
        let res_u = (self.mean_residual * 1.5).clamp(0.0, 0.6);
        let step_u = 0.04 * (n.saturating_sub(1) as f64);
        let uncertainty = (0.45 * base_u + 0.40 * res_u + step_u).clamp(0.05, 0.95);
        (out, uncertainty)
    }

    pub fn record_residual(&mut self, observed: &BTreeMap<String, f64>) -> f64 {
        let (pred, _) = self.predict_with_uncertainty(1);
        let err = rms_diff(&pred, observed);
        self.mean_residual = 0.85 * self.mean_residual + 0.15 * err;
        if !self.last_linear.is_empty() && !self.last_attn.is_empty() {
            let e_lin = rms_diff(&self.last_linear, observed);
            let e_att = rms_diff(&self.last_attn, observed);
            if e_att + 1e-9 < e_lin {
                self.blend = (self.blend + 0.05).clamp(0.1, 0.9);
            } else {
                self.blend = (self.blend - 0.05).clamp(0.1, 0.9);
            }
        }
        self.attn.update_readout(&pred, observed, 0.05);
        err
    }
}

pub struct HybridWorldPredictor {
    model: RefCell<HybridWorldModel>,
}

impl Default for HybridWorldPredictor {
    fn default() -> Self {
        Self::new()
    }
}

impl HybridWorldPredictor {
    pub fn new() -> Self {
        Self {
            model: RefCell::new(HybridWorldModel::default()),
        }
    }

    pub fn blend(&self) -> f64 {
        self.model.borrow().blend
    }
}

impl Predictor for HybridWorldPredictor {
    fn predict(
        &self,
        observations: &[Observation],
        memory_ctx: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<Prediction>> {
        let mut obs_vec = BTreeMap::new();
        for (idx, obs) in observations.iter().enumerate() {
            if let Some(num) = obs.content.as_f64() {
                if num.is_finite() {
                    obs_vec.insert(obs.source.clone(), num);
                }
            } else if let Some(obj) = obs.content.as_object() {
                for (k, v) in obj {
                    if let Some(num) = v.as_f64() {
                        if num.is_finite() {
                            obs_vec.insert(format!("{}:{k}", obs.source), num);
                        }
                    }
                }
            } else {
                obs_vec.insert(format!("obs_{idx}"), 1.0);
            }
        }
        if obs_vec.is_empty() {
            obs_vec.insert("signal".into(), 0.5);
        }

        let effect = memory_ctx
            .get("cognitive_effect")
            .and_then(|v| serde_json::from_value::<RuleEffect>(v.clone()).ok())
            .unwrap_or_default();

        let mut model = self.model.borrow_mut();
        let _err = model.record_residual(&obs_vec);
        model.update(&obs_vec, 0.85);
        let (expected_map, raw_u) = model.predict_with_uncertainty(1);
        let uncertainty = (raw_u * effect.error_damping).max(effect.min_uncertainty);

        Ok(vec![Prediction {
            expected: serde_json::json!({
                "state": expected_map,
                "blend": model.blend,
                "mean_residual": model.mean_residual,
                "damping": effect.error_damping,
            }),
            uncertainty,
            horizon_secs: 1.0,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hybrid_model_updates_blend_and_bounds_uncertainty() {
        let mut hm = HybridWorldModel::default();
        for step in 0..8 {
            let mut obs = BTreeMap::new();
            obs.insert("x".into(), (step as f64) * 0.1);
            hm.record_residual(&obs);
            hm.update(&obs, 0.9);
        }
        let (pred, u) = hm.predict_with_uncertainty(1);
        assert!(pred.contains_key("x"));
        assert!((0.05..=0.95).contains(&u));
        assert!((0.1..=0.9).contains(&hm.blend));
    }

    #[test]
    fn hybrid_predictor_implements_l0_trait() {
        let predictor = HybridWorldPredictor::new();
        let item = Observation {
            timestamp: crate::now_utc(),
            content: serde_json::json!({"x": 0.4}),
            confidence: 0.9,
            source: "sensor".into(),
            modality: crate::types::Modality::Text,
        };
        let obs = vec![item];
        let preds = predictor
            .predict(&obs, &HashMap::new())
            .expect("predict ok");
        assert_eq!(preds.len(), 1);
        assert!((0.05..=0.95).contains(&preds[0].uncertainty));
    }
}
