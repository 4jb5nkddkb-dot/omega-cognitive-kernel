use crate::types::MetaMetrics;

pub struct EmergenceDetector {
    pub history: Vec<MetaMetrics>,
    pub window: usize,
    pub pred_threshold: f64,
    pub self_ref_threshold: f64,
    /// Minimum steps between successful triggers (cooldown / hysteresis).
    pub cooldown_steps: usize,
    /// Steps remaining in cooldown after a trigger.
    pub cooldown_remaining: usize,
}

impl Default for EmergenceDetector {
    fn default() -> Self {
        Self {
            history: Vec::new(),
            window: 4,
            pred_threshold: 0.58,
            self_ref_threshold: 0.50,
            cooldown_steps: 5,
            cooldown_remaining: 0,
        }
    }
}

impl EmergenceDetector {
    /// Returns true only when thresholds are met *and* cooldown has expired.
    pub fn should_trigger(&mut self, metrics: &MetaMetrics) -> bool {
        self.history.push(metrics.clone());
        if self.history.len() > self.window * 4 {
            let drain_to = self.history.len() - self.window * 2;
            self.history.drain(0..drain_to);
        }

        if self.cooldown_remaining > 0 {
            self.cooldown_remaining -= 1;
            return false;
        }

        if self.history.len() < self.window {
            return false;
        }

        let recent = &self.history[self.history.len() - self.window..];
        let avg_pred: f64 =
            recent.iter().map(|m| m.predictive_power).sum::<f64>() / recent.len() as f64;
        let avg_self: f64 =
            recent.iter().map(|m| m.self_ref_stability).sum::<f64>() / recent.len() as f64;

        let triggered = avg_pred > self.pred_threshold
            && avg_self > self.self_ref_threshold
            && metrics.predictive_power >= avg_pred * 0.98;

        if triggered {
            self.cooldown_remaining = self.cooldown_steps;
        }
        triggered
    }

    /// Soft exit signal: predictive power or self-ref dropped significantly.
    pub fn should_exit(&self, metrics: &MetaMetrics) -> bool {
        if self.history.len() < self.window {
            return false;
        }
        let recent = &self.history[self.history.len() - self.window..];
        let avg_pred: f64 =
            recent.iter().map(|m| m.predictive_power).sum::<f64>() / recent.len() as f64;
        let avg_self: f64 =
            recent.iter().map(|m| m.self_ref_stability).sum::<f64>() / recent.len() as f64;

        metrics.predictive_power < avg_pred * 0.85
            || metrics.self_ref_stability < avg_self * 0.80
            || metrics.predictive_power < 0.40
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MetaMetrics;
    use chrono::Utc;

    fn metrics(pred: f64, self_ref: f64) -> MetaMetrics {
        MetaMetrics {
            complexity: 1.0,
            consistency: 0.7,
            predictive_power: pred,
            self_ref_stability: self_ref,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn does_not_trigger_with_insufficient_history() {
        let mut det = EmergenceDetector::default();
        assert!(!det.should_trigger(&metrics(0.9, 0.9)));
        assert!(!det.should_trigger(&metrics(0.9, 0.9)));
    }

    #[test]
    fn triggers_when_thresholds_met() {
        let mut det = EmergenceDetector {
            history: Vec::new(),
            window: 3,
            pred_threshold: 0.5,
            self_ref_threshold: 0.5,
            cooldown_steps: 0, // disable for this test
            cooldown_remaining: 0,
        };
        for _ in 0..3 {
            let _ = det.should_trigger(&metrics(0.8, 0.8));
        }
        let triggered = det.should_trigger(&metrics(0.85, 0.85));
        assert!(triggered);
    }

    #[test]
    fn cooldown_blocks_immediate_retrigger() {
        let mut det = EmergenceDetector {
            history: Vec::new(),
            window: 2,
            pred_threshold: 0.5,
            self_ref_threshold: 0.5,
            cooldown_steps: 3,
            cooldown_remaining: 0,
        };
        // Fill history
        let _ = det.should_trigger(&metrics(0.9, 0.9));
        let _ = det.should_trigger(&metrics(0.9, 0.9));
        assert!(det.should_trigger(&metrics(0.9, 0.9))); // first trigger
        // Next calls should be blocked by cooldown
        assert!(!det.should_trigger(&metrics(0.9, 0.9)));
        assert!(!det.should_trigger(&metrics(0.9, 0.9)));
    }

    #[test]
    fn exit_when_metrics_drop() {
        let mut det = EmergenceDetector {
            history: Vec::new(),
            window: 3,
            pred_threshold: 0.5,
            self_ref_threshold: 0.5,
            cooldown_steps: 0,
            cooldown_remaining: 0,
        };
        for _ in 0..4 {
            let _ = det.should_trigger(&metrics(0.85, 0.85));
        }
        assert!(det.should_exit(&metrics(0.3, 0.3)));
    }
}
