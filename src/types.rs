use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Modality {
    Text,
    Image,
    Audio,
    Structured,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub timestamp: DateTime<Utc>,
    pub modality: Modality,
    pub content: serde_json::Value,
    pub confidence: f64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub expected: serde_json::Value,
    pub uncertainty: f64,
    pub horizon_secs: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub goal: String,
    pub priority: f64,
    pub bias_from_memory: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub name: String,
    pub params: HashMap<String, serde_json::Value>,
    pub constraints_checked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trajectory {
    pub observations: Vec<Observation>,
    pub predictions: Vec<Prediction>,
    pub actions: Vec<Action>,
    pub outcomes: Vec<serde_json::Value>,
    pub prediction_errors: Vec<f64>,
    pub meta: HashMap<String, serde_json::Value>,
}

/// Structured, safely-applied effect of a cognitive patch.
/// This is what actually changes L0 behavior — not the `code` string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEffect {
    /// Multiplies prediction error / uncertainty (1.0 = no change, lower = better).
    pub error_damping: f64,
    /// Additive preference scores for action names.
    pub action_preference: HashMap<String, f64>,
    /// Floor for reported uncertainty after damping.
    pub min_uncertainty: f64,
    /// Additive score bias toward "explore" (negative favors exploit/stabilize).
    pub explore_bonus: f64,
    /// Soft threshold for treating uncertainty as "high" (higher = more tolerant).
    pub risk_tolerance: f64,
}

impl Default for RuleEffect {
    fn default() -> Self {
        Self {
            error_damping: 1.0,
            action_preference: HashMap::new(),
            min_uncertainty: 0.05,
            explore_bonus: 0.0,
            risk_tolerance: 0.45,
        }
    }
}

impl RuleEffect {
    pub const ALLOWED_ACTIONS: &'static [&'static str] =
        &["noop", "observe", "explore", "exploit", "stabilize"];

    pub fn sanitize(mut self) -> Self {
        self.error_damping = self.error_damping.clamp(0.2, 1.5);
        self.min_uncertainty = self.min_uncertainty.clamp(0.01, 0.5);
        self.explore_bonus = self.explore_bonus.clamp(-1.5, 1.5);
        self.risk_tolerance = self.risk_tolerance.clamp(0.15, 0.85);
        self.action_preference.retain(|k, v| {
            if !Self::ALLOWED_ACTIONS.contains(&k.as_str()) {
                return false;
            }
            *v = v.clamp(-5.0, 5.0);
            true
        });
        self
    }

    pub fn validate_bounds(&self) -> Option<String> {
        if !(0.15..=2.0).contains(&self.error_damping) {
            return Some(format!(
                "error_damping {:.3} outside [0.15, 2.0]",
                self.error_damping
            ));
        }
        if !(0.0..=1.0).contains(&self.min_uncertainty) {
            return Some(format!(
                "min_uncertainty {:.3} outside [0.0, 1.0]",
                self.min_uncertainty
            ));
        }
        if !(-2.0..=2.0).contains(&self.explore_bonus) {
            return Some(format!(
                "explore_bonus {:.3} outside [-2.0, 2.0]",
                self.explore_bonus
            ));
        }
        if !(0.1..=0.9).contains(&self.risk_tolerance) {
            return Some(format!(
                "risk_tolerance {:.3} outside [0.1, 0.9]",
                self.risk_tolerance
            ));
        }
        for (k, v) in &self.action_preference {
            if !Self::ALLOWED_ACTIONS.contains(&k.as_str()) {
                return Some(format!("unknown action preference key: {k}"));
            }
            if !(-10.0..=10.0).contains(v) {
                return Some(format!("action preference {k}={v} outside [-10, 10]"));
            }
        }
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitivePatch {
    pub patch_id: Uuid,
    pub description: String,
    pub code: String,
    pub target_module: String,
    pub expected_effect: String,
    pub risk_level: f64,
    pub created_at: DateTime<Utc>,
    pub effect: RuleEffect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaMetrics {
    pub complexity: f64,
    pub consistency: f64,
    pub predictive_power: f64,
    pub self_ref_stability: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct SystemState {
    pub active_code: HashMap<String, String>,
    pub highest_attractor: String,
    pub generative_base: Option<String>,
    pub last_trajectory_summary: Option<String>,
    pub avg_prediction_error: f64,
    pub total_steps: u64,
}
