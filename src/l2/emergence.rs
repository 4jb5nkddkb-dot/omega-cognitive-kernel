use crate::types::MetaMetrics;

pub struct EmergenceDetector {
    pub history: Vec<MetaMetrics>,
    pub window: usize,
    pub pred_threshold: f64,
    pub self_ref_threshold: f64,
}

impl Default for EmergenceDetector {
    fn default() -> Self {
        Self {
            history: Vec::new(),
            window: 4,
            pred_threshold: 0.58,
            self_ref_threshold: 0.50,
        }
    }
}

impl EmergenceDetector {
    pub fn should_trigger(&mut self, metrics: &MetaMetrics) -> bool {
        self.history.push(metrics.clone());
        if self.history.len() > self.window * 4 {
            let drain_to = self.history.len() - self.window * 2;
            self.history.drain(0..drain_to);
        }
        if self.history.len() < self.window {
            return false;
        }

        let recent = &self.history[self.history.len() - self.window..];
        let avg_pred: f64 =
            recent.iter().map(|m| m.predictive_power).sum::<f64>() / recent.len() as f64;
        let avg_self: f64 =
            recent.iter().map(|m| m.self_ref_stability).sum::<f64>() / recent.len() as f64;

        avg_pred > self.pred_threshold
            && avg_self > self.self_ref_threshold
            && metrics.predictive_power >= avg_pred * 0.98
    }
}
