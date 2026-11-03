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
}

impl Default for RuleEffect {
    fn default() -> Self {
        Self {
            error_damping: 1.0,
            action_preference: HashMap::new(),
            min_uncertainty: 0.05,
        }
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
    /// Machine-readable effect applied on commit (S1 consequence path).
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
