//! Minimal task environment: nonstationary 2-armed bandit.
//! External reward is written back into strategy memory so the task loop is closed.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::api::Kernel;
use crate::error::Result;
use crate::l0::bandit_decision::BanditUcbDecisionMaker;
use crate::l0::decision::DecisionMaker;
use hashbrown::HashMap;
use serde::{Serialize, Serializer};
use std::env::temp_dir;

fn serialize_score_breakdown<S>(
    values: &HashMap<String, f64>,
    serializer: S,
) -> core::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut entries: Vec<_> = values.iter().collect();
    entries.sort_unstable_by_key(|(key, _)| *key);
    serializer.collect_map(entries)
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct L2ExpertMetrics {
    pub committed: u64,
    pub applied: u64,
    pub not_applied: u64,
    pub reward_sum: f64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct L2PipelineDiagnostics {
    pub observations: u64,
    pub spectral_active: u64,
    pub proposals: u64,
    pub dry_runs: u64,
    /// Candidate arm changed relative to KEEP_L0 on the same pre-action snapshot.
    pub candidate_changes: u64,
    /// Selected arm changed relative to KEEP_L0 on that same snapshot.
    pub selected_changes: u64,
    /// Counterfactual changed candidate but L0 safety/hysteresis held the arm.
    pub candidate_changes_not_selected: u64,
    pub history_discounts: u64,
    pub spectral_rejections: u64,
    pub lyapunov_rejections: u64,
    pub cooldown_rejections: u64,
    pub confidence_rejections: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct L2DiagnosticSample {
    pub step: u64,
    pub selected_arm: usize,
    pub optimal_arm: usize,
    pub reward: f64,
    pub spectral_bias: f64,
    pub normalized_energy: f64,
    pub spectral_dc_drift: f64,
    pub convergence_residual: f64,
    pub controller_residual: f64,
    pub alpha: f64,
    pub beta: f64,
    pub confidence: f64,
    pub proposed_expert: Option<usize>,
    pub applied: bool,
    pub dry_run_baseline_arm: Option<usize>,
    pub dry_run_treatment_arm: Option<usize>,
    pub dry_run_baseline_candidate: Option<usize>,
    pub dry_run_treatment_candidate: Option<usize>,
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct CorrelationDiagnostic {
    pub samples: usize,
    pub pearson: Option<f64>,
    pub spearman: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spearman_bootstrap_ci95: Option<[f64; 2]>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct L2SignalDiagnostics {
    pub bias_to_next_reward: CorrelationDiagnostic,
    pub residual_to_next_error: CorrelationDiagnostic,
    pub bias_to_regime_change: CorrelationDiagnostic,
    pub bias_to_next_optimal_arm: CorrelationDiagnostic,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct L2ExperimentSettings {
    pub spectral_control: bool,
    pub purity_prior: bool,
    pub reverse_brake: bool,
    pub history_discount: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BanditReport {
    pub scenario: String,
    pub episodes: u32,
    pub steps_per_episode: u32,
    pub mean_reward: f64,
    /// Mean external reward after UCB's error, rollback, and switching costs.
    /// This is a learning diagnostic, not a substitute for external reward.
    pub mean_adjusted_reward: f64,
    pub best_arm_rate: f64,
    pub final_damping: f64,
    pub final_explore_bonus: f64,
    pub final_risk_tolerance: f64,
    pub error_decreased: bool,
    pub patches_accepted: u64,
    pub seed: u64,
    // Strategy-decay observability
    pub strategy_count: usize,
    pub idle_strategy_count: usize,
    pub mean_strategy_weight: f64,
    pub mean_idle_steps: f64,
    pub max_idle_steps: u32,
    pub mean_idle_weight: f64,
    pub mean_active_weight: f64,
    pub initial_avg_error: f64,
    pub final_avg_error: f64,
    pub fixed_point_enters: usize,
    pub fixed_point_exits: usize,
    pub max_fixed_point_dwell: u64,
    pub l2_nudges: usize,
    pub l2_proposals: usize,
    pub l2_commits: usize,
    pub action_switches: usize,
    pub post_commit_action_changes: usize,
    pub post_commit_arm_changes: usize,
    pub distinct_actions: usize,
    pub mean_l2_consequence_score: f64,
    #[serde(serialize_with = "serialize_score_breakdown")]
    pub final_score_breakdown: HashMap<String, f64>,
    pub final_alignment_score: f64,
    pub recovery_steps: Option<u32>,
    pub l1_rollback_count: u64,
    pub l1_blocked_by_l2_count: u64,
    pub l2_rollback_count: u64,
    pub dayan_converged: bool,
    pub final_dayan_residual: f64,
    pub dwell_hold_count: u64,
    pub margin_hold_count: u64,
    pub active_steps: u64,
    pub committed_policies_applied: u64,
    pub expert_metrics: [L2ExpertMetrics; 4],
    pub lyapunov_vetoes: u64,
    pub meta_cache_hits: u64,
    pub recovery_latencies: Vec<u32>,
    pub warm_recovery_latencies: Vec<u32>,
    pub mean_residual_improvement: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline: Option<L2PipelineDiagnostics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<Vec<L2DiagnosticSample>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signals: Option<L2SignalDiagnostics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experiment: Option<L2ExperimentSettings>,
}

#[derive(Debug, Clone, Serialize)]
pub struct L2BenchmarkAggregate {
    pub runs: u32,
    pub mean_reward: f64,
    pub mean_adjusted_reward: f64,
    pub best_arm_rate: f64,
    pub error_decrease_rate: f64,
    pub mean_initial_error: f64,
    pub mean_final_error: f64,
    pub mean_error_delta: f64,
    pub mean_patches_accepted: f64,
    pub mean_fixed_point_enters: f64,
    pub mean_fixed_point_exits: f64,
    pub mean_max_fixed_point_dwell: f64,
    pub mean_l2_nudges: f64,
    pub mean_l2_proposals: f64,
    pub mean_l2_commits: f64,
    pub mean_action_switches: f64,
    pub mean_post_commit_action_changes: f64,
    pub mean_post_commit_arm_changes: f64,
    pub mean_distinct_actions: f64,
    pub mean_l2_consequence_score: f64,
    #[serde(serialize_with = "serialize_score_breakdown")]
    pub mean_final_score_breakdown: HashMap<String, f64>,
    pub mean_final_alignment_score: f64,
    pub mean_recovery_steps: Option<f64>,
    pub recovery_rate: f64,
    pub mean_l1_rollback_count: f64,
    pub mean_l1_blocked_by_l2_count: f64,
    pub mean_l2_rollback_count: f64,
    pub dayan_convergence_rate: f64,
    pub mean_final_dayan_residual: f64,
    pub mean_dwell_hold_count: f64,
    pub mean_margin_hold_count: f64,
    pub mean_active_steps: f64,
    pub mean_committed_policies_applied: f64,
    pub mean_expert_committed: [f64; 4],
    pub mean_expert_applied: [f64; 4],
    pub mean_expert_not_applied: [f64; 4],
    pub mean_expert_reward: [Option<f64>; 4],
    pub mean_lyapunov_vetoes: f64,
    pub mean_meta_cache_hits: f64,
    pub recovery_event_rate: f64,
    pub mean_warm_recovery_steps: Option<f64>,
    pub mean_residual_improvement: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct PairedMeanDifference {
    pub mean: f64,
    pub ci95_low: Option<f64>,
    pub ci95_high: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct L2PairedRun {
    pub seed: u64,
    pub baseline: BanditReport,
    pub treatment: BanditReport,
    pub delta_mean_reward: f64,
    pub delta_best_arm_rate: f64,
    pub delta_error_delta: f64,
    pub treatment_influenced_l0: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct L2BenchmarkReport {
    pub scenario: String,
    pub episodes: u32,
    pub steps_per_episode: u32,
    pub runs: u32,
    pub seed: u64,
    pub baseline_l2_disabled: L2BenchmarkAggregate,
    pub treatment_l2_enabled: L2BenchmarkAggregate,
    pub delta_mean_reward: f64,
    pub delta_best_arm_rate: f64,
    pub delta_error_decrease_rate: f64,
    pub paired_reward_difference: PairedMeanDifference,
    pub paired_best_arm_difference: PairedMeanDifference,
    pub paired_reward_bootstrap: PairedMeanDifference,
    pub paired_best_arm_bootstrap: PairedMeanDifference,
    pub l0_influenced_runs: u32,
    /// Per-seed evidence for diagnosing differences that occur without a commit.
    pub paired_runs: Vec<L2PairedRun>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FactorialArm {
    L0,
    L1,
    L2,
    L1L2,
}

impl FactorialArm {
    pub const ALL: [Self; 4] = [Self::L0, Self::L1, Self::L2, Self::L1L2];

    fn index(self) -> usize {
        match self {
            Self::L0 => 0,
            Self::L1 => 1,
            Self::L2 => 2,
            Self::L1L2 => 3,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::L0 => "L0",
            Self::L1 => "L1",
            Self::L2 => "L2",
            Self::L1L2 => "L1+L2",
        }
    }

    fn l1_enabled(self) -> bool {
        matches!(self, Self::L1 | Self::L1L2)
    }

    fn l2_enabled(self) -> bool {
        matches!(self, Self::L2 | Self::L1L2)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct FactorialEffect {
    pub reward: PairedMeanDifference,
    pub reward_bootstrap: PairedMeanDifference,
    pub best_arm_rate: PairedMeanDifference,
    pub adjusted_reward: PairedMeanDifference,
}

#[derive(Debug, Clone, Serialize)]
pub struct FactorialRun {
    pub seed: u64,
    pub arms: [BanditReport; 4],
    pub l1_reward_delta: f64,
    pub l2_reward_delta: f64,
    pub joint_reward_delta: f64,
    pub interaction_reward_delta: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FactorialBenchmarkReport {
    pub scenario: String,
    pub episodes: u32,
    pub steps_per_episode: u32,
    pub runs: u32,
    pub seed: u64,
    pub arm_order: [FactorialArm; 4],
    pub aggregates: [L2BenchmarkAggregate; 4],
    pub l1_effect: FactorialEffect,
    pub l2_effect: FactorialEffect,
    pub joint_effect: FactorialEffect,
    pub interaction_effect: FactorialEffect,
    pub paired_runs: Vec<FactorialRun>,
}

pub const L2_GAIN_SCENARIOS: [&str; 4] =
    ["stable", "nonstationary", "regime_shift", "cyclic_shift"];
pub const L2_GAIN_DISCOVERY_SEED: u64 = 20261115;
pub const L2_GAIN_VALIDATION_SEED: u64 = 20270001;
pub const L2_GAIN_DEFAULT_RUNS: u32 = 30;
pub const L2_GAIN_DEFAULT_EPISODES: u32 = 12;
pub const L2_GAIN_DEFAULT_STEPS: u32 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum L2GainSeedSet {
    Discovery,
    Validation,
}

#[derive(Debug, Clone, Serialize)]
pub struct L2GainScenarioAssessment {
    pub reward_ci95_lower_positive: bool,
    pub stable_loss_within_tolerance: bool,
    pub executed_interventions: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct L2GainAssessment {
    pub primary_scenario: String,
    pub safety_scenario: String,
    pub stable_loss_tolerance: f64,
    pub mechanism_precondition_met: bool,
    pub discovery_primary_passed: bool,
    pub validation_reproduced: bool,
    pub discovery_safety_passed: bool,
    pub validation_safety_passed: bool,
    pub reproducible_gain: bool,
    pub conclusion: String,
    pub discovery: L2GainScenarioAssessment,
    pub validation: L2GainScenarioAssessment,
}

#[derive(Debug, Clone, Serialize)]
pub struct L2GainMatrixReport {
    pub schema_version: u32,
    pub design: String,
    pub runs_per_scenario: u32,
    pub episodes: u32,
    pub steps_per_episode: u32,
    pub scenarios: [&'static str; 4],
    pub discovery_seed: u64,
    pub validation_seed: u64,
    pub kernel_runs: u64,
    pub decision_steps: u64,
    pub reports: Vec<FactorialBenchmarkReport>,
    pub assessment: L2GainAssessment,
}

fn assess_gain_scenario(report: &FactorialBenchmarkReport) -> L2GainScenarioAssessment {
    let executed_interventions = report
        .paired_runs
        .iter()
        .map(|run| run.arms[FactorialArm::L2.index()].committed_policies_applied)
        .sum();
    L2GainScenarioAssessment {
        reward_ci95_lower_positive: report
            .l2_effect
            .reward
            .ci95_low
            .is_some_and(|low| low > 0.0),
        stable_loss_within_tolerance: report
            .l2_effect
            .reward
            .ci95_low
            .is_some_and(|low| low >= -0.01),
        executed_interventions,
    }
}

fn gain_conclusion(assessment: &L2GainAssessment) -> String {
    if assessment.reproducible_gain {
        "L2-only reward gain reproduced within this fixture and seed matrix".into()
    } else if !assessment.mechanism_precondition_met {
        "No executed L2 intervention; reward differences cannot demonstrate an L2 gain".into()
    } else if assessment.discovery_primary_passed && !assessment.validation_reproduced {
        "Discovery gain did not reproduce on the independent validation seeds".into()
    } else {
        "The preregistered reward criterion did not demonstrate a reproducible L2 gain".into()
    }
}

fn assess_l2_gain_matrix(reports: &[FactorialBenchmarkReport]) -> Result<L2GainAssessment> {
    if reports.len() != L2_GAIN_SCENARIOS.len() * 2 {
        return Err(crate::error::OmegaError::Memory(format!(
            "gain assessment requires {} reports, got {}",
            L2_GAIN_SCENARIOS.len() * 2,
            reports.len()
        )));
    }
    for seed_set in [L2GainSeedSet::Discovery, L2GainSeedSet::Validation] {
        for scenario in L2_GAIN_SCENARIOS {
            let expected_seed = match seed_set {
                L2GainSeedSet::Discovery => L2_GAIN_DISCOVERY_SEED,
                L2GainSeedSet::Validation => L2_GAIN_VALIDATION_SEED,
            };
            let matching: Vec<_> = reports
                .iter()
                .filter(|report| report.scenario == scenario && report.seed == expected_seed)
                .collect();
            if matching.len() != 1 {
                return Err(crate::error::OmegaError::Memory(format!(
                    "gain assessment requires one {seed_set:?} report for {scenario}"
                )));
            }
            let report = matching[0];
            if report.runs < 2 || report.paired_runs.len() != report.runs as usize {
                return Err(crate::error::OmegaError::Memory(
                    "gain report has incomplete paired seeds".into(),
                ));
            }
            for (index, pair) in report.paired_runs.iter().enumerate() {
                if pair.seed != expected_seed.wrapping_add(index as u64) {
                    return Err(crate::error::OmegaError::Memory(
                        "gain report has missing, duplicate, or reordered seeds".into(),
                    ));
                }
            }
        }
    }
    let find = |seed_set: L2GainSeedSet, scenario: &str| {
        let expected_seed = match seed_set {
            L2GainSeedSet::Discovery => L2_GAIN_DISCOVERY_SEED,
            L2GainSeedSet::Validation => L2_GAIN_VALIDATION_SEED,
        };
        reports
            .iter()
            .find(|report| report.scenario == scenario && report.seed == expected_seed)
            .ok_or_else(|| {
                crate::error::OmegaError::Memory(format!(
                    "missing {seed_set:?} gain report for {scenario}"
                ))
            })
    };
    let discovery = find(L2GainSeedSet::Discovery, "regime_shift")?;
    let validation = find(L2GainSeedSet::Validation, "regime_shift")?;
    let discovery_stable = find(L2GainSeedSet::Discovery, "stable")?;
    let validation_stable = find(L2GainSeedSet::Validation, "stable")?;
    let discovery = assess_gain_scenario(discovery);
    let validation = assess_gain_scenario(validation);
    let discovery_stable = assess_gain_scenario(discovery_stable);
    let validation_stable = assess_gain_scenario(validation_stable);
    let mechanism_precondition_met =
        discovery.executed_interventions > 0 && validation.executed_interventions > 0;
    let mut assessment = L2GainAssessment {
        primary_scenario: "regime_shift".into(),
        safety_scenario: "stable".into(),
        stable_loss_tolerance: 0.01,
        mechanism_precondition_met,
        discovery_primary_passed: discovery.reward_ci95_lower_positive,
        validation_reproduced: validation.reward_ci95_lower_positive,
        discovery_safety_passed: discovery_stable.stable_loss_within_tolerance,
        validation_safety_passed: validation_stable.stable_loss_within_tolerance,
        reproducible_gain: false,
        conclusion: String::new(),
        discovery,
        validation,
    };
    assessment.reproducible_gain = assessment.mechanism_precondition_met
        && assessment.discovery_primary_passed
        && assessment.validation_reproduced
        && assessment.discovery_safety_passed
        && assessment.validation_safety_passed;
    assessment.conclusion = gain_conclusion(&assessment);
    Ok(assessment)
}

fn factorial_differences<F>(runs: &[FactorialRun], value: F) -> [Vec<f64>; 4]
where
    F: Fn(&BanditReport) -> f64,
{
    let mut differences = core::array::from_fn(|_| Vec::with_capacity(runs.len()));
    for run in runs {
        let values = [
            value(&run.arms[0]),
            value(&run.arms[1]),
            value(&run.arms[2]),
            value(&run.arms[3]),
        ];
        differences[0].push(values[1] - values[0]);
        differences[1].push(values[2] - values[0]);
        differences[2].push(values[3] - values[0]);
        differences[3].push(values[3] - values[2] - values[1] + values[0]);
    }
    differences
}

fn factorial_effects(runs: &[FactorialRun], seed: u64) -> [FactorialEffect; 4] {
    let reward = factorial_differences(runs, |report| report.mean_reward);
    let best_arm = factorial_differences(runs, |report| report.best_arm_rate);
    let adjusted = factorial_differences(runs, |report| report.mean_adjusted_reward);
    core::array::from_fn(|effect| FactorialEffect {
        reward: paired_difference(&reward[effect]),
        reward_bootstrap: bootstrap_difference(
            &reward[effect],
            seed ^ 0x4c314c32 ^ (effect as u64),
        ),
        best_arm_rate: paired_difference(&best_arm[effect]),
        adjusted_reward: paired_difference(&adjusted[effect]),
    })
}

struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed | 1 }
    }

    fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.state >> 8) as f64 / (1u64 << 56) as f64
    }
}

fn arm_for_action(action: &str) -> usize {
    match action {
        "stabilize" | "exploit" => 0,
        "explore" => 1,
        _ => 0,
    }
}

pub fn run_bandit_task(
    kernel: &mut Kernel,
    episodes: u32,
    steps_per_episode: u32,
) -> Result<BanditReport> {
    run_bandit_task_seeded(kernel, episodes, steps_per_episode, 405123076)
}

pub fn run_bandit_task_seeded(
    kernel: &mut Kernel,
    episodes: u32,
    steps_per_episode: u32,
    seed: u64,
) -> Result<BanditReport> {
    run_bandit_task_seeded_scenario(
        kernel,
        episodes,
        steps_per_episode,
        seed,
        "nonstationary",
        None,
    )
}

fn run_bandit_task_seeded_scenario<D>(
    kernel: &mut Kernel<
        crate::l0::observer::MultiModalObserver,
        D,
        crate::l0::executor::SandboxedExecutor,
    >,
    episodes: u32,
    steps_per_episode: u32,
    seed: u64,
    scenario: &str,
    experiment: Option<L2ExperimentSettings>,
) -> Result<BanditReport>
where
    D: DecisionMaker,
{
    let mut rng = Rng::new(seed);
    let mut total_reward = 0.0;
    let mut total_adjusted_reward = 0.0;
    let mut best_hits = 0u32;
    let mut total_choices = 0u32;
    let mut patches = 0u64;
    let mut l2_nudges = 0usize;
    let mut l2_proposals = 0usize;
    let mut l2_commits = 0usize;
    let mut action_switches = 0usize;
    let mut post_commit_action_changes = 0usize;
    let mut post_commit_arm_changes = 0usize;
    let mut distinct_actions = std::collections::HashSet::new();
    let mut previous_action: Option<String> = None;
    let mut pending_proposal: Option<(usize, String, usize)> = None;
    let mut max_fixed_point_dwell = 0u64;
    let mut consequence_sum = 0.0;
    let mut early_err = None;
    let mut final_score_breakdown = HashMap::new();
    let mut recovery_steps = None;
    let mut recovery_streak = 0;
    let mut l1_rollback_count = 0;
    let mut l1_blocked_by_l2_count = 0;
    let mut l2_rollback_count = 0;
    let mut final_dayan_residual = 1.0;
    let mut dwell_hold_count = 0;
    let mut margin_hold_count = 0;
    let mut active_steps = 0;
    let mut committed_policies_applied = 0;
    let mut expert_metrics: [L2ExpertMetrics; 4] =
        core::array::from_fn(|_| L2ExpertMetrics::default());
    let mut lyapunov_vetoes = 0;
    let mut meta_cache_hits = 0;
    let mut recovery_latencies = Vec::new();
    let mut warm_recovery_latencies = Vec::new();
    let mut recovery_started_at = None;
    let mut recovery_warm = false;
    let mut recovery_used_commit = false;
    let mut last_warm_commit: Option<crate::types::L2Policy> = None;
    let mut pipeline = L2PipelineDiagnostics::default();
    let mut diagnostic_samples = Vec::new();
    let collect_diagnostics = experiment.is_some();
    let mut residual_improvement = 0.0;
    let mut initial_dayan_residual = None;
    let mut previous_dayan_residual = None;
    let mut dayan_decreased_steps = 0u64;
    let total_steps = episodes.saturating_mul(steps_per_episode);
    let shift_step = total_steps / 2;
    let recovery_target = total_steps.saturating_sub(shift_step) / 3;

    for ep in 0..episodes {
        // Non-stationary: best arm flips halfway (and once more late if many episodes).
        let best = match scenario {
            "stable" | "fixedpoint" => 0,
            "degraded" if ep >= episodes / 2 => 0,
            _ if ep >= episodes / 2 && !(episodes >= 4 && ep >= (episodes * 3) / 4) => 1,
            _ => 0,
        };
        if scenario == "degraded" && ep == episodes / 2 {
            kernel.inner.l0.memory.cas.avg_prediction_error = 0.9;
        }
        if scenario == "fixedpoint" {
            kernel.inner.l0.memory.cas.avg_prediction_error =
                if ep < episodes / 2 { 0.2 } else { 0.9 };
        }
        let mut ep_reward = 0.0;

        for step_in_episode in 0..steps_per_episode {
            let global_step = ep * steps_per_episode + step_in_episode;
            let best = if scenario == "cyclic_shift" {
                (global_step / 12) as usize % 2
            } else if matches!(scenario, "regime_shift" | "nonstationary")
                && global_step >= shift_step
            {
                if scenario == "nonstationary" && global_step >= (total_steps * 3) / 4 {
                    0
                } else {
                    1
                }
            } else {
                best
            };
            let mut before_err = kernel.inner.l0.memory.cas.avg_prediction_error;
            if matches!(scenario, "regime_shift" | "cyclic_shift")
                && ((scenario == "regime_shift" && global_step >= shift_step)
                    || (scenario == "cyclic_shift" && global_step % 12 < 3))
            {
                // Inject observation noise before learning and preserve the
                // same pre-step error for the consequence calculation.
                kernel.inner.l0.memory.cas.avg_prediction_error =
                    if global_step == shift_step { 0.9 } else { 0.65 };
                before_err = kernel.inner.l0.memory.cas.avg_prediction_error;
            }
            // Capture the state before L0 consumes the one-shot policy. Both
            // dry-runs must use this exact snapshot: a post-action snapshot has
            // already changed the incumbent/dwell state and cannot attribute
            // this action to L2.
            let dry_run = if collect_diagnostics {
                let applied = kernel.inner.l2.pending_policy.effective();
                kernel
                    .inner
                    .l0
                    .decision
                    .bandit_snapshot()
                    .map(|mut snapshot| {
                        // The snapshot's policy was consumed by the previous action.
                        // run_once replaces it with pending_policy before deciding.
                        snapshot.l2_policy = crate::types::L2Policy::default();
                        let mut baseline = BanditUcbDecisionMaker::default();
                        baseline.restore(snapshot.clone());
                        let (baseline, _) =
                            baseline.dry_run_policy(crate::types::L2Policy::default());
                        let mut treatment = BanditUcbDecisionMaker::default();
                        treatment.restore(snapshot);
                        let (treatment, _) = treatment.dry_run_policy(applied);
                        (baseline, treatment)
                    })
            } else {
                None
            };
            let r = kernel
                .step("maintain coherent self-model omega dayan while maximizing bandit reward")?;
            let after_err = kernel.inner.l0.memory.cas.avg_prediction_error;
            if early_err.is_none() {
                early_err = Some(after_err);
            }
            if r.l1.patched {
                patches += 1;
            }
            let l1_rolled_back = kernel.observe_l1_consequence(after_err - before_err);
            let l1_rollback = l1_rolled_back || r.l1.rollback_effect.is_some();
            let applied = r.l2.applied_policy;
            if let Some(logged_policy) = r
                .trajectory
                .actions
                .first()
                .and_then(|action| action.params.get("l2_policy"))
                .and_then(|value| {
                    serde_json::from_value::<crate::types::L2Policy>(value.clone()).ok()
                })
            {
                debug_assert_eq!(applied, logged_policy);
            }
            let kl_policy = r
                .trajectory
                .actions
                .first()
                .and_then(|action| {
                    serde_json::from_value::<[f64; 2]>(action.params.get("kl_policy")?.clone()).ok()
                })
                .unwrap_or([1.0, 0.0]);
            if r.l2.effect_nudge.is_some() {
                l2_nudges += 1;
            }
            if r.l2.modulation_proposed {
                l2_proposals += 1;
            }
            if r.l2.modulation_committed {
                l2_commits += 1;
            }
            if collect_diagnostics {
                pipeline.observations += 1;
                pipeline.spectral_active += u64::from(
                    r.l2.spectral_output
                        .is_some_and(|output| output.state == crate::l2::L2State::Active),
                );
                pipeline.history_discounts += u64::from(
                    r.l2.spectral_output
                        .is_some_and(|output| output.regime_detected),
                );
                match r.l2.rejection_reason.as_deref() {
                    Some("spectral_gate") => pipeline.spectral_rejections += 1,
                    Some("lyapunov_veto") => pipeline.lyapunov_rejections += 1,
                    Some("commit_cooldown") => pipeline.cooldown_rejections += 1,
                    Some("commit_threshold") | Some("trust_or_confidence_zero") => {
                        pipeline.confidence_rejections += 1
                    }
                    _ => {}
                }
            }
            max_fixed_point_dwell = max_fixed_point_dwell.max(r.l2.dwell_steps);
            final_dayan_residual = r.l2.dayan_residual;
            residual_improvement += previous_dayan_residual
                .map(|previous| previous - r.l2.dayan_residual)
                .unwrap_or(0.0);
            dayan_decreased_steps += u64::from(
                previous_dayan_residual
                    .map(|previous| r.l2.dayan_residual < previous)
                    .unwrap_or(false),
            );
            initial_dayan_residual.get_or_insert(r.l2.dayan_residual);
            previous_dayan_residual = Some(r.l2.dayan_residual);
            let l2_policy_applied =
                r.l2.decision
                    .as_ref()
                    .map(|decision| decision.l2_policy_applied)
                    .unwrap_or(applied.committed);
            if collect_diagnostics {
                pipeline.dry_runs += 1;
                if let Some((baseline, treatment)) = &dry_run {
                    let candidate_changed = baseline.candidate_arm != treatment.candidate_arm;
                    let selected_changed = baseline.selected_arm != treatment.selected_arm;
                    pipeline.candidate_changes += u64::from(candidate_changed);
                    pipeline.selected_changes += u64::from(selected_changed);
                    pipeline.candidate_changes_not_selected +=
                        u64::from(candidate_changed && !selected_changed);
                }
                pipeline.proposals += u64::from(r.l2.modulation_committed);
                let spec = r.l2.spectral_output;
                diagnostic_samples.push(L2DiagnosticSample {
                    step: kernel.inner.total_steps,
                    selected_arm: r.l2.decision.as_ref().map(|d| d.selected_arm).unwrap_or(0),
                    optimal_arm: best,
                    reward: 0.0,
                    spectral_bias: spec.map(|s| s.bias).unwrap_or(0.0),
                    normalized_energy: spec.map(|s| s.normalized_energy).unwrap_or(0.0),
                    spectral_dc_drift: spec.map(|s| s.dc_drift).unwrap_or(0.0),
                    convergence_residual: spec.map(|s| s.convergence_residual).unwrap_or(0.0),
                    controller_residual: r.l2.dayan_residual,
                    alpha: r.l2.alpha,
                    beta: r.l2.beta,
                    confidence: kernel.inner.l2.controller.model_confidence,
                    proposed_expert: r.l2.modulation_committed.then_some(r.l2.l2_policy.expert),
                    applied: l2_policy_applied,
                    dry_run_baseline_arm: dry_run
                        .as_ref()
                        .map(|(baseline, _)| baseline.selected_arm),
                    dry_run_treatment_arm: dry_run
                        .as_ref()
                        .map(|(_, treatment)| treatment.selected_arm),
                    dry_run_baseline_candidate: dry_run
                        .as_ref()
                        .map(|(baseline, _)| baseline.candidate_arm),
                    dry_run_treatment_candidate: dry_run
                        .as_ref()
                        .map(|(_, treatment)| treatment.candidate_arm),
                    rejection_reason: r.l2.rejection_reason.clone(),
                });
            }
            active_steps += u64::from(r.l2.gate_state == crate::l2::control::L2GateState::Active);
            committed_policies_applied += u64::from(l2_policy_applied);
            lyapunov_vetoes += u64::from(r.l2.lyapunov_veto);
            meta_cache_hits += u64::from(r.l2.meta_cache_hit);
            if let Some(decision) = &r.l2.decision {
                dwell_hold_count += u64::from(decision.dwell_suppressed);
                margin_hold_count += u64::from(decision.margin_suppressed);
            }
            if !r.l2.score_breakdown.is_empty() {
                final_score_breakdown = r.l2.score_breakdown.clone();
            }
            let action = r
                .trajectory
                .actions
                .first()
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "observe".into());
            let arm = arm_for_action(&action);
            distinct_actions.insert(action.clone());
            if previous_action
                .as_deref()
                .map(|prev| prev != action)
                .unwrap_or(false)
            {
                action_switches += 1;
            }
            if let Some((commit_step, before_action, before_arm)) = pending_proposal.take() {
                debug_assert_eq!(commit_step + 1, kernel.inner.total_steps as usize);
                if l2_policy_applied {
                    if action != before_action {
                        post_commit_action_changes += 1;
                    }
                    if arm != before_arm {
                        post_commit_arm_changes += 1;
                    }
                }
            }
            if r.l2.modulation_committed {
                pending_proposal = Some((kernel.inner.total_steps as usize, action.clone(), arm));
            }
            previous_action = Some(action.clone());
            let p = if scenario == "degraded" && ep >= episodes / 2 {
                if arm == best {
                    0.45
                } else {
                    0.35
                }
            } else if arm == best {
                0.85
            } else {
                0.25
            };
            let reward = if rng.next_f64() < p { 1.0 } else { 0.0 };
            if let Some(sample) = diagnostic_samples.last_mut() {
                sample.reward = reward;
            }
            if let Some(proposed) = r
                .trajectory
                .actions
                .first()
                .and_then(|action| action.params.get("l2_proposed_policy"))
                .and_then(|value| {
                    serde_json::from_value::<crate::types::L2Policy>(value.clone()).ok()
                })
                .filter(|policy| policy.committed && policy.expert < 4)
            {
                let metrics = &mut expert_metrics[proposed.expert];
                metrics.committed += 1;
                if l2_policy_applied {
                    metrics.applied += 1;
                    metrics.reward_sum += reward;
                } else {
                    metrics.not_applied += 1;
                }
            }

            let rollback_count = u64::from(l1_rollback) + u64::from(r.l1.safety_degraded);
            let switch_cost =
                r.l2.decision
                    .as_ref()
                    .filter(|decision| decision.switched && decision.selected_arm == arm)
                    .map(|decision| decision.switch_cost_applied)
                    .unwrap_or(0.0);
            let rollback_penalty = kernel
                .inner
                .l0
                .decision
                .bandit_config()
                .map(|config| config.rollback_penalty)
                .unwrap_or(0.0);
            let adjusted_reward = reward
                - 0.25 * (after_err - before_err)
                - rollback_penalty * rollback_count as f64
                - switch_cost;
            total_adjusted_reward += adjusted_reward;
            kernel.inner.l0.memory.record_reward(&action, reward);
            // External reward is separated from error improvement and explicit
            // L1 safety costs so a rollback cannot disappear behind a good error.
            kernel.inner.l0.decision.observe_with_feedback(
                arm,
                reward,
                crate::types::DecisionFeedback {
                    l1_rollback,
                    safety_degradation: r.l1.safety_degraded,
                    error_delta: after_err - before_err,
                },
            );
            debug_assert!(
                kernel.inner.l0.decision.bandit_config().is_none()
                    || kernel
                        .inner
                        .l0
                        .decision
                        .l2_decision()
                        .and_then(|decision| decision.adjusted_reward)
                        == Some(adjusted_reward),
                "reported net reward must equal the UCB learning feedback"
            );
            kernel.observe_l2_feedback(
                reward,
                after_err,
                l1_rollback,
                kernel.inner.l2.last_consequence_score,
                kl_policy,
            );
            let consequence_score = 0.60 * (reward - 0.5) - 0.25 * (after_err - before_err);
            consequence_sum += consequence_score;
            kernel.observe_l2_consequence(reward - 0.5, after_err - before_err, 0.0);
            if kernel.inner.l2.last_consequence_score < -0.05
                && kernel.inner.l2.rollback_effect.is_some()
            {
                l2_rollback_count += 1;
            }
            l1_blocked_by_l2_count += u64::from(r.l1_blocked_by_l2);
            l1_rollback_count += u64::from(l1_rollback);
            if r.l2.modulation_committed {
                last_warm_commit = kernel
                    .inner
                    .l2
                    .controller
                    .meta_cache_hit
                    .then_some(r.l2.l2_policy);
            }
            recovery_used_commit |= last_warm_commit == Some(applied) && applied.committed;
            if matches!(scenario, "regime_shift" | "cyclic_shift") {
                let transition = (scenario == "regime_shift" && global_step == shift_step)
                    || (scenario == "cyclic_shift" && global_step > 0 && global_step % 12 == 0);
                if transition {
                    recovery_started_at = Some(global_step);
                    recovery_streak = 0;
                    recovery_warm = false;
                    recovery_used_commit = false;
                    last_warm_commit = None;
                }
                let target = if scenario == "regime_shift" {
                    recovery_target
                } else {
                    3
                };
                if let Some(start) = recovery_started_at {
                    recovery_warm |= kernel.inner.l2.controller.meta_cache_hit;
                    recovery_streak = if arm == best { recovery_streak + 1 } else { 0 };
                    if recovery_streak >= target.max(1) {
                        let latency = global_step - start + 1;
                        recovery_latencies.push(latency);
                        if recovery_warm && recovery_used_commit {
                            warm_recovery_latencies.push(latency);
                        }
                        recovery_steps.get_or_insert(latency);
                        recovery_started_at = None;
                    }
                }
            }

            ep_reward += reward;
            total_reward += reward;
            total_choices += 1;
            if arm == best {
                best_hits += 1;
            }
        }

        kernel.audit_task("bandit2", ep_reward, steps_per_episode)?;
    }

    let final_err = kernel.inner.l0.memory.cas.avg_prediction_error;
    let initial_err = early_err.unwrap_or(final_err);
    let effect = &kernel.inner.l0.memory.cognitive_effect;
    let idle_threshold = kernel.inner.l0.learner.decay_idle_threshold;
    let st = kernel.inner.l0.memory.strategy.stats(idle_threshold);
    let audit = kernel.audit_summary()?;
    let status = kernel.status();

    let signals = collect_diagnostics.then(|| signal_diagnostics(&diagnostic_samples));
    Ok(BanditReport {
        scenario: scenario.into(),
        episodes,
        steps_per_episode,
        mean_reward: if total_choices == 0 {
            0.0
        } else {
            total_reward / total_choices as f64
        },
        mean_adjusted_reward: total_adjusted_reward / total_choices.max(1) as f64,
        best_arm_rate: if total_choices == 0 {
            0.0
        } else {
            best_hits as f64 / total_choices as f64
        },
        final_damping: effect.error_damping,
        final_explore_bonus: effect.explore_bonus,
        final_risk_tolerance: effect.risk_tolerance,
        error_decreased: final_err < initial_err,
        patches_accepted: patches,
        seed,
        strategy_count: st.strategy_count,
        idle_strategy_count: st.idle_count,
        mean_strategy_weight: st.mean_weight,
        mean_idle_steps: st.mean_idle_steps,
        max_idle_steps: st.max_idle_steps,
        mean_idle_weight: st.mean_idle_weight,
        mean_active_weight: st.mean_active_weight,
        initial_avg_error: initial_err,
        final_avg_error: final_err,
        fixed_point_enters: audit.fixed_point_enters,
        fixed_point_exits: audit.fixed_point_exits,
        max_fixed_point_dwell,
        l2_nudges,
        l2_proposals,
        l2_commits,
        action_switches,
        post_commit_action_changes,
        post_commit_arm_changes,
        distinct_actions: distinct_actions.len(),
        mean_l2_consequence_score: if total_choices == 0 {
            0.0
        } else {
            consequence_sum / total_choices as f64
        },
        final_score_breakdown,
        final_alignment_score: status.last_alignment_score,
        recovery_steps,
        l1_rollback_count,
        l1_blocked_by_l2_count,
        l2_rollback_count,
        dayan_converged: final_dayan_residual <= 0.18
            && dayan_decreased_steps > 0
            && initial_dayan_residual
                .map(|initial| final_dayan_residual <= initial)
                .unwrap_or(false),
        final_dayan_residual,
        dwell_hold_count,
        margin_hold_count,
        active_steps,
        committed_policies_applied,
        expert_metrics,
        lyapunov_vetoes,
        meta_cache_hits,
        recovery_latencies,
        warm_recovery_latencies,
        mean_residual_improvement: residual_improvement / total_choices.max(1) as f64,
        experiment,
        pipeline: collect_diagnostics.then_some(pipeline),
        diagnostics: collect_diagnostics.then_some(diagnostic_samples),
        signals,
    })
}

fn aggregate(reports: &[BanditReport]) -> L2BenchmarkAggregate {
    let shift_events = reports
        .first()
        .map(|r| match r.scenario.as_str() {
            "regime_shift" => 1,
            "cyclic_shift" => {
                r.episodes
                    .saturating_mul(r.steps_per_episode)
                    .saturating_sub(1)
                    / 12
            }
            _ => 0,
        })
        .unwrap_or(0);
    let runs = reports.len() as u32;
    if reports.is_empty() {
        return L2BenchmarkAggregate {
            runs,
            mean_reward: 0.0,
            mean_adjusted_reward: 0.0,
            best_arm_rate: 0.0,
            error_decrease_rate: 0.0,
            mean_initial_error: 0.0,
            mean_final_error: 0.0,
            mean_error_delta: 0.0,
            mean_patches_accepted: 0.0,
            mean_fixed_point_enters: 0.0,
            mean_fixed_point_exits: 0.0,
            mean_max_fixed_point_dwell: 0.0,
            mean_l2_nudges: 0.0,
            mean_l2_proposals: 0.0,
            mean_l2_commits: 0.0,
            mean_action_switches: 0.0,
            mean_post_commit_action_changes: 0.0,
            mean_post_commit_arm_changes: 0.0,
            mean_distinct_actions: 0.0,
            mean_l2_consequence_score: 0.0,
            mean_final_score_breakdown: HashMap::new(),
            mean_final_alignment_score: 0.0,
            mean_recovery_steps: None,
            recovery_rate: 0.0,
            mean_l1_rollback_count: 0.0,
            mean_l1_blocked_by_l2_count: 0.0,
            mean_l2_rollback_count: 0.0,
            dayan_convergence_rate: 0.0,
            mean_final_dayan_residual: 0.0,
            mean_dwell_hold_count: 0.0,
            mean_margin_hold_count: 0.0,
            mean_active_steps: 0.0,
            mean_committed_policies_applied: 0.0,
            mean_expert_committed: [0.0; 4],
            mean_expert_applied: [0.0; 4],
            mean_expert_not_applied: [0.0; 4],
            mean_expert_reward: [None; 4],
            mean_lyapunov_vetoes: 0.0,
            mean_meta_cache_hits: 0.0,
            recovery_event_rate: 0.0,
            mean_warm_recovery_steps: None,
            mean_residual_improvement: 0.0,
        };
    }
    let n = reports.len() as f64;
    let mean = |f: fn(&BanditReport) -> f64| reports.iter().map(f).sum::<f64>() / n;
    let mean_expert = |f: fn(&L2ExpertMetrics) -> f64, expert: usize| {
        reports
            .iter()
            .map(|report| f(&report.expert_metrics[expert]))
            .sum::<f64>()
            / n
    };
    L2BenchmarkAggregate {
        runs,
        mean_reward: mean(|r| r.mean_reward),
        mean_adjusted_reward: mean(|r| r.mean_adjusted_reward),
        best_arm_rate: mean(|r| r.best_arm_rate),
        error_decrease_rate: reports.iter().filter(|r| r.error_decreased).count() as f64 / n,
        mean_initial_error: mean(|r| r.initial_avg_error),
        mean_final_error: mean(|r| r.final_avg_error),
        mean_error_delta: mean(|r| r.final_avg_error - r.initial_avg_error),
        mean_patches_accepted: mean(|r| r.patches_accepted as f64),
        mean_fixed_point_enters: mean(|r| r.fixed_point_enters as f64),
        mean_fixed_point_exits: mean(|r| r.fixed_point_exits as f64),
        mean_max_fixed_point_dwell: mean(|r| r.max_fixed_point_dwell as f64),
        mean_l2_nudges: mean(|r| r.l2_nudges as f64),
        mean_l2_proposals: mean(|r| r.l2_proposals as f64),
        mean_l2_commits: mean(|r| r.l2_commits as f64),
        mean_action_switches: mean(|r| r.action_switches as f64),
        mean_post_commit_action_changes: mean(|r| r.post_commit_action_changes as f64),
        mean_post_commit_arm_changes: mean(|r| r.post_commit_arm_changes as f64),
        mean_distinct_actions: mean(|r| r.distinct_actions as f64),
        mean_l2_consequence_score: mean(|r| r.mean_l2_consequence_score),
        mean_final_score_breakdown: {
            // Iterating each HashMap directly makes floating-point addition
            // depend on its randomized hash seed. Accumulate in key order, then
            // sum keys in the caller's deterministic report/seed order.
            let mut keys: Vec<&String> = reports
                .iter()
                .flat_map(|report| report.final_score_breakdown.keys())
                .collect();
            keys.sort_unstable();
            keys.dedup();
            keys.into_iter()
                .map(|key| {
                    let sum = reports
                        .iter()
                        .filter_map(|report| report.final_score_breakdown.get(key))
                        .sum::<f64>();
                    (key.clone(), sum / n)
                })
                .collect()
        },
        mean_final_alignment_score: mean(|r| r.final_alignment_score),
        mean_recovery_steps: {
            let recovered: Vec<u32> = reports.iter().filter_map(|r| r.recovery_steps).collect();
            (!recovered.is_empty())
                .then(|| recovered.iter().sum::<u32>() as f64 / recovered.len() as f64)
        },
        recovery_rate: reports
            .iter()
            .filter(|r| r.recovery_steps.is_some())
            .count() as f64
            / n,
        mean_l1_rollback_count: mean(|r| r.l1_rollback_count as f64),
        mean_l1_blocked_by_l2_count: mean(|r| r.l1_blocked_by_l2_count as f64),
        mean_l2_rollback_count: mean(|r| r.l2_rollback_count as f64),
        dayan_convergence_rate: reports.iter().filter(|r| r.dayan_converged).count() as f64 / n,
        mean_final_dayan_residual: mean(|r| r.final_dayan_residual),
        mean_dwell_hold_count: mean(|r| r.dwell_hold_count as f64),
        mean_margin_hold_count: mean(|r| r.margin_hold_count as f64),
        mean_active_steps: mean(|r| r.active_steps as f64),
        mean_committed_policies_applied: mean(|r| r.committed_policies_applied as f64),
        mean_expert_committed: core::array::from_fn(|expert| {
            mean_expert(|metrics| metrics.committed as f64, expert)
        }),
        mean_expert_applied: core::array::from_fn(|expert| {
            mean_expert(|metrics| metrics.applied as f64, expert)
        }),
        mean_expert_not_applied: core::array::from_fn(|expert| {
            mean_expert(|metrics| metrics.not_applied as f64, expert)
        }),
        mean_expert_reward: core::array::from_fn(|expert| {
            let applied = reports
                .iter()
                .map(|r| r.expert_metrics[expert].applied)
                .sum::<u64>();
            (applied > 0).then(|| {
                reports
                    .iter()
                    .map(|r| r.expert_metrics[expert].reward_sum)
                    .sum::<f64>()
                    / applied as f64
            })
        }),
        mean_lyapunov_vetoes: mean(|r| r.lyapunov_vetoes as f64),
        mean_meta_cache_hits: mean(|r| r.meta_cache_hits as f64),
        recovery_event_rate: if shift_events == 0 {
            0.0
        } else {
            mean(|r| r.recovery_latencies.len() as f64) / shift_events as f64
        },
        mean_warm_recovery_steps: {
            let recovered: Vec<u32> = reports
                .iter()
                .flat_map(|r| &r.warm_recovery_latencies)
                .copied()
                .collect();
            (!recovered.is_empty())
                .then(|| recovered.iter().sum::<u32>() as f64 / recovered.len() as f64)
        },
        mean_residual_improvement: mean(|r| r.mean_residual_improvement),
    }
}

fn cleanup_benchmark_files(state_path: &std::path::Path) {
    if let Some(parent) = state_path.parent() {
        let _ = std::fs::remove_dir_all(parent);
    }
}

fn paired_difference(differences: &[f64]) -> PairedMeanDifference {
    let n = differences.len() as f64;
    let mean = differences.iter().sum::<f64>() / n;
    if differences.len() < 2 {
        return PairedMeanDifference {
            mean,
            ci95_low: None,
            ci95_high: None,
        };
    }
    let variance = differences
        .iter()
        .map(|difference| (difference - mean).powi(2))
        .sum::<f64>()
        / (n - 1.0);
    // Student t critical values (df=1..30, two-sided 95%); normal for df>30.
    const T95: [f64; 30] = [
        12.706, 4.303, 3.182, 2.776, 2.571, 2.447, 2.365, 2.306, 2.262, 2.228, 2.201, 2.179, 2.160,
        2.145, 2.131, 2.120, 2.110, 2.101, 2.093, 2.086, 2.080, 2.074, 2.069, 2.064, 2.060, 2.056,
        2.052, 2.048, 2.045, 2.042,
    ];
    let df = differences.len() - 1;
    let critical = T95.get(df - 1).copied().unwrap_or(1.96);
    let half_width = critical * (variance / n).sqrt();
    PairedMeanDifference {
        mean,
        ci95_low: Some(mean - half_width),
        ci95_high: Some(mean + half_width),
    }
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn pearson(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() < 2 || x.len() != y.len() {
        return None;
    }
    let mx = mean(x);
    let my = mean(y);
    let numerator = x
        .iter()
        .zip(y)
        .map(|(a, b)| (a - mx) * (b - my))
        .sum::<f64>();
    let sx = x.iter().map(|a| (a - mx).powi(2)).sum::<f64>().sqrt();
    let sy = y.iter().map(|b| (b - my).powi(2)).sum::<f64>().sqrt();
    (sx > 0.0 && sy > 0.0).then_some(numerator / (sx * sy))
}

fn ranks(values: &[f64]) -> Vec<f64> {
    let mut sorted: Vec<(usize, f64)> = values.iter().copied().enumerate().collect();
    sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));
    let mut ranks = vec![0.0; values.len()];
    let mut start = 0;
    while start < sorted.len() {
        let mut end = start + 1;
        while end < sorted.len() && sorted[end].1 == sorted[start].1 {
            end += 1;
        }
        let rank = (start + end - 1) as f64 / 2.0 + 1.0;
        for &(index, _) in &sorted[start..end] {
            ranks[index] = rank;
        }
        start = end;
    }
    ranks
}

fn correlation(pairs: &[(f64, f64)]) -> CorrelationDiagnostic {
    let x: Vec<f64> = pairs.iter().map(|pair| pair.0).collect();
    let y: Vec<f64> = pairs.iter().map(|pair| pair.1).collect();
    let spearman = pearson(&ranks(&x), &ranks(&y));
    let spearman_bootstrap_ci95 = if pairs.len() >= 8 {
        let mut rng = Rng::new(0x53504541524d414e);
        let mut bootstrap = Vec::with_capacity(1_000);
        for _ in 0..1_000 {
            let mut sample_x = Vec::with_capacity(pairs.len());
            let mut sample_y = Vec::with_capacity(pairs.len());
            for _ in 0..pairs.len() {
                let index = ((rng.next_f64() * pairs.len() as f64) as usize).min(pairs.len() - 1);
                sample_x.push(x[index]);
                sample_y.push(y[index]);
            }
            if let Some(value) = pearson(&ranks(&sample_x), &ranks(&sample_y)) {
                bootstrap.push(value);
            }
        }
        bootstrap.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
        (bootstrap.len() >= 20).then(|| {
            [
                bootstrap[bootstrap.len() / 40],
                bootstrap[bootstrap.len() * 39 / 40],
            ]
        })
    } else {
        None
    };
    CorrelationDiagnostic {
        samples: pairs.len(),
        pearson: pearson(&x, &y),
        spearman,
        spearman_bootstrap_ci95,
    }
}

fn signal_diagnostics(samples: &[L2DiagnosticSample]) -> L2SignalDiagnostics {
    let mut reward = Vec::new();
    let mut residual = Vec::new();
    let mut regime = Vec::new();
    let mut optimal = Vec::new();
    for pair in samples.windows(2) {
        let current = &pair[0];
        let next = &pair[1];
        if current.normalized_energy > 0.0 {
            reward.push((current.spectral_bias, next.reward));
        }
        optimal.push((current.spectral_bias, next.optimal_arm as f64));
        residual.push((current.convergence_residual, next.convergence_residual));
        regime.push((
            current.spectral_bias,
            f64::from(current.optimal_arm != next.optimal_arm),
        ));
    }
    L2SignalDiagnostics {
        bias_to_next_reward: correlation(&reward),
        residual_to_next_error: correlation(&residual),
        bias_to_regime_change: correlation(&regime),
        bias_to_next_optimal_arm: correlation(&optimal),
    }
}

fn bootstrap_difference(differences: &[f64], seed: u64) -> PairedMeanDifference {
    let mean_difference = mean(differences);
    if differences.len() < 2 {
        return PairedMeanDifference {
            mean: mean_difference,
            ci95_low: None,
            ci95_high: None,
        };
    }
    let mut rng = Rng::new(seed);
    let mut means = Vec::with_capacity(2_000);
    let mut sample = vec![0.0; differences.len()];
    for _ in 0..2_000 {
        for value in &mut sample {
            let index =
                ((rng.next_f64() * differences.len() as f64) as usize).min(differences.len() - 1);
            *value = differences[index];
        }
        means.push(mean(&sample));
    }
    means.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    PairedMeanDifference {
        mean: mean_difference,
        ci95_low: Some(means[(means.len() as f64 * 0.025) as usize]),
        ci95_high: Some(means[(means.len() as f64 * 0.975) as usize]),
    }
}

/// Compare identical seeded bandit runs with L2 interventions disabled/enabled.
/// The benchmark is deterministic and reports deltas rather than treating a
/// single successful run as evidence of improvement.
pub fn run_l2_benchmark(
    runs: u32,
    episodes: u32,
    steps_per_episode: u32,
    seed: u64,
) -> Result<L2BenchmarkReport> {
    run_l2_benchmark_scenario("nonstationary", runs, episodes, steps_per_episode, seed)
}

/// Run the paired L2 benchmark under a named deterministic environment.
pub fn run_l2_benchmark_scenario(
    scenario: &str,
    runs: u32,
    episodes: u32,
    steps_per_episode: u32,
    seed: u64,
) -> Result<L2BenchmarkReport> {
    let negative_reward_required = if scenario == "stable" { 5 } else { 3 };
    let (commit_threshold, commit_cooldown) = if scenario == "degraded" {
        (0.50, 3)
    } else {
        // Confidence of the one-step residual model, not anomaly severity.
        (0.35, 3)
    };
    run_l2_benchmark_scenario_with_gate(
        scenario,
        runs,
        episodes,
        steps_per_episode,
        seed,
        commit_threshold,
        commit_cooldown,
        negative_reward_required,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn run_l2_benchmark_scenario_with_gate(
    scenario: &str,
    runs: u32,
    episodes: u32,
    steps_per_episode: u32,
    seed: u64,
    commit_threshold: f64,
    commit_cooldown_steps: u64,
    negative_reward_required: u32,
    experiment: Option<L2ExperimentSettings>,
) -> Result<L2BenchmarkReport> {
    let scenario = match scenario {
        "stable" | "nonstationary" | "degraded" | "fixedpoint" | "regime_shift"
        | "cyclic_shift" => scenario,
        other => {
            return Err(crate::error::OmegaError::Config(format!(
                "unknown benchmark scenario: {other}"
            )))
        }
    };
    let runs = runs.max(1);
    let mut paired_runs = Vec::with_capacity(runs as usize);

    for i in 0..runs {
        let run_seed = seed.wrapping_add(i as u64);
        let base_dir = temp_dir().join(format!(
            "omega-l2-benchmark-{}-{}-base",
            std::process::id(),
            crate::new_uuid()
        ));
        let treatment_dir = temp_dir().join(format!(
            "omega-l2-benchmark-{}-{}-l2",
            std::process::id(),
            crate::new_uuid()
        ));
        std::fs::create_dir_all(&base_dir)
            .map_err(|error| crate::error::OmegaError::Persist(error.to_string()))?;
        std::fs::create_dir_all(&treatment_dir)
            .map_err(|error| crate::error::OmegaError::Persist(error.to_string()))?;
        let base_path = base_dir.join("state.json");
        let treatment_path = treatment_dir.join("state.json");

        let mut base = Kernel::with_bandit_ucb(&base_path);
        let mut l2 = Kernel::with_bandit_ucb(&treatment_path);
        for branch in [&mut base, &mut l2] {
            branch.autosave = false;
            branch.auto_checkpoint_on_patch = false;
            branch.dump_trajectory = false;
            branch.inner.l1.self_modify_enabled = true;
            branch
                .inner
                .l1
                .store
                .active
                .insert("cognitive_rules".into(), "self-model bias damp rule".into());
            branch.inner.l2.detector.window = 2;
            branch.inner.l2.detector.pred_threshold = 0.30;
            branch.inner.l2.detector.self_ref_threshold = 0.30;
            branch.inner.l2.detector.cooldown_steps = 0;
            branch.inner.l2.commit_threshold = commit_threshold.clamp(0.0, 1.0);
            branch.inner.l2.commit_cooldown_steps = commit_cooldown_steps;
            branch.inner.l2.negative_reward_required = negative_reward_required.max(1);
            branch.inner.l2.configure_spectral_control(
                experiment
                    .as_ref()
                    .is_some_and(|settings| settings.spectral_control),
            );
            branch.inner.l2.spectral_gate.configure_ablation(
                experiment
                    .as_ref()
                    .is_some_and(|settings| settings.purity_prior),
                experiment
                    .as_ref()
                    .is_some_and(|settings| settings.reverse_brake),
                experiment
                    .as_ref()
                    .is_some_and(|settings| settings.history_discount),
            );
            if matches!(scenario, "regime_shift" | "cyclic_shift") {
                let mut state = branch.inner.l0.decision.snapshot();
                state.discount_factor = 0.85;
                branch.inner.l0.decision.restore(state);
            }
        }
        // Only the intervention permission differs. Every other policy,
        // feedback, persistence, and environment parameter is paired.
        base.set_l2_enabled(false);
        l2.set_l2_enabled(true);

        // Both arms start from the same coherent policy context. This makes
        // fixed-point entry observable while preserving a fair intervention
        // comparison: only L2 is disabled in the baseline.
        base.inner
            .l1
            .store
            .active
            .insert("cognitive_rules".into(), "self-model bias damp rule".into());
        base.set_l2_enabled(false);
        base.inner.l1.self_modify_enabled = true;
        if matches!(scenario, "regime_shift" | "cyclic_shift") {
            let mut state = base.inner.l0.decision.snapshot();
            state.discount_factor = 0.85;
            base.inner.l0.decision.restore(state);
        }
        let base_report = run_bandit_task_seeded_scenario(
            &mut base,
            episodes,
            steps_per_episode,
            run_seed,
            scenario,
            None,
        )?;
        cleanup_benchmark_files(&base_path);
        let treatment_report = run_bandit_task_seeded_scenario(
            &mut l2,
            episodes,
            steps_per_episode,
            run_seed,
            scenario,
            experiment.clone(),
        )?;
        cleanup_benchmark_files(&treatment_path);
        paired_runs.push(L2PairedRun {
            seed: run_seed,
            delta_mean_reward: treatment_report.mean_reward - base_report.mean_reward,
            delta_best_arm_rate: treatment_report.best_arm_rate - base_report.best_arm_rate,
            delta_error_delta: (treatment_report.final_avg_error
                - treatment_report.initial_avg_error)
                - (base_report.final_avg_error - base_report.initial_avg_error),
            treatment_influenced_l0: treatment_report.committed_policies_applied > 0
                || treatment_report.l2_nudges > 0,
            baseline: base_report,
            treatment: treatment_report,
        });
    }

    let baseline: Vec<_> = paired_runs.iter().map(|run| run.baseline.clone()).collect();
    let treatment: Vec<_> = paired_runs
        .iter()
        .map(|run| run.treatment.clone())
        .collect();
    let baseline = aggregate(&baseline);
    let treatment = aggregate(&treatment);
    let reward_differences: Vec<_> = paired_runs
        .iter()
        .map(|run| run.delta_mean_reward)
        .collect();
    let best_arm_differences: Vec<_> = paired_runs
        .iter()
        .map(|run| run.delta_best_arm_rate)
        .collect();
    let paired_reward_difference = paired_difference(&reward_differences);
    let paired_best_arm_difference = paired_difference(&best_arm_differences);
    let l0_influenced_runs = paired_runs
        .iter()
        .filter(|run| run.treatment_influenced_l0)
        .count() as u32;
    Ok(L2BenchmarkReport {
        scenario: scenario.into(),
        episodes,
        steps_per_episode,
        runs,
        seed,
        delta_mean_reward: treatment.mean_reward - baseline.mean_reward,
        delta_best_arm_rate: treatment.best_arm_rate - baseline.best_arm_rate,
        delta_error_decrease_rate: treatment.error_decrease_rate - baseline.error_decrease_rate,
        paired_reward_difference,
        paired_best_arm_difference,
        paired_reward_bootstrap: bootstrap_difference(&reward_differences, seed ^ 0x4c32),
        paired_best_arm_bootstrap: bootstrap_difference(&best_arm_differences, seed ^ 0x4241),
        l0_influenced_runs,
        baseline_l2_disabled: baseline,
        treatment_l2_enabled: treatment,
        paired_runs,
    })
}

/// Run a 2x2 factorial comparison of independent L1/L2 intervention gates.
/// Arm order is L0, L1-only, L2-only, and L1+L2. Each position in a seeded
/// environment consumes one random number, providing common random numbers
/// across arms without changing the existing task environment.
#[allow(clippy::too_many_arguments)]
pub fn run_l1_l2_factorial_benchmark(
    scenario: &str,
    runs: u32,
    episodes: u32,
    steps_per_episode: u32,
    seed: u64,
    commit_threshold: f64,
    commit_cooldown_steps: u64,
    negative_reward_required: u32,
) -> Result<FactorialBenchmarkReport> {
    if !matches!(
        scenario,
        "stable" | "nonstationary" | "degraded" | "fixedpoint" | "regime_shift" | "cyclic_shift"
    ) {
        return Err(crate::error::OmegaError::Config(format!(
            "unknown benchmark scenario: {scenario}"
        )));
    }
    let runs = runs.max(1);
    let mut paired_runs = Vec::with_capacity(runs as usize);
    for index in 0..runs {
        let run_seed = seed.wrapping_add(index as u64);
        let mut arm_reports = Vec::with_capacity(FactorialArm::ALL.len());
        for arm in FactorialArm::ALL {
            let directory = temp_dir().join(format!(
                "omega-l1-l2-factorial-{}-{}-{}",
                std::process::id(),
                crate::new_uuid(),
                arm.label()
            ));
            std::fs::create_dir_all(&directory)
                .map_err(|error| crate::error::OmegaError::Persist(error.to_string()))?;
            let path = directory.join("state.json");
            let mut kernel = Kernel::with_bandit_ucb(&path);
            kernel.autosave = false;
            kernel.auto_checkpoint_on_patch = false;
            kernel.dump_trajectory = false;
            kernel.inner.l1.self_modify_enabled = arm.l1_enabled();
            kernel.inner.l1.shadow_mode = false;
            kernel
                .inner
                .l1
                .store
                .active
                .insert("cognitive_rules".into(), "self-model bias damp rule".into());
            kernel.inner.l2.detector.window = 2;
            kernel.inner.l2.detector.pred_threshold = 0.30;
            kernel.inner.l2.detector.self_ref_threshold = 0.30;
            kernel.inner.l2.detector.cooldown_steps = 0;
            kernel.inner.l2.commit_threshold = commit_threshold.clamp(0.0, 1.0);
            kernel.inner.l2.commit_cooldown_steps = commit_cooldown_steps;
            kernel.inner.l2.negative_reward_required = negative_reward_required.max(1);
            kernel.inner.l2.configure_spectral_control(false);
            if matches!(scenario, "regime_shift" | "cyclic_shift") {
                let mut state = kernel.inner.l0.decision.snapshot();
                state.discount_factor = 0.85;
                kernel.inner.l0.decision.restore(state);
            }
            kernel.set_l2_enabled(arm.l2_enabled());
            let report = run_bandit_task_seeded_scenario(
                &mut kernel,
                episodes,
                steps_per_episode,
                run_seed,
                scenario,
                None,
            );
            cleanup_benchmark_files(&path);
            arm_reports.push(report?);
        }
        let arms: [BanditReport; 4] = arm_reports.try_into().map_err(|_| {
            crate::error::OmegaError::Memory("factorial benchmark requires four arms".into())
        })?;
        let l1_reward_delta = arms[1].mean_reward - arms[0].mean_reward;
        let l2_reward_delta = arms[2].mean_reward - arms[0].mean_reward;
        let joint_reward_delta = arms[3].mean_reward - arms[0].mean_reward;
        let interaction_reward_delta =
            arms[3].mean_reward - arms[2].mean_reward - arms[1].mean_reward + arms[0].mean_reward;
        paired_runs.push(FactorialRun {
            seed: run_seed,
            arms,
            l1_reward_delta,
            l2_reward_delta,
            joint_reward_delta,
            interaction_reward_delta,
        });
    }
    let aggregates = FactorialArm::ALL.map(|arm| {
        let reports: Vec<_> = paired_runs
            .iter()
            .map(|run| run.arms[arm.index()].clone())
            .collect();
        aggregate(&reports)
    });
    let [l1_effect, l2_effect, joint_effect, interaction_effect] =
        factorial_effects(&paired_runs, seed);
    Ok(FactorialBenchmarkReport {
        scenario: scenario.into(),
        episodes,
        steps_per_episode,
        runs,
        seed,
        arm_order: FactorialArm::ALL,
        aggregates,
        l1_effect,
        l2_effect,
        joint_effect,
        interaction_effect,
        paired_runs,
    })
}

/// Execute the preregistered discovery/validation L2-only gain matrix. The
/// factorial reports retain L1 and interaction diagnostics, but only L2-L0
/// participates in the gain verdict. No controller setting is tuned here.
pub fn run_l2_gain_matrix(
    runs: u32,
    episodes: u32,
    steps_per_episode: u32,
) -> Result<L2GainMatrixReport> {
    if runs < 2 || episodes == 0 || steps_per_episode == 0 {
        return Err(crate::error::OmegaError::Config(
            "gain matrix requires at least two paired seeds and nonzero horizon".into(),
        ));
    }
    let mut reports = Vec::with_capacity(L2_GAIN_SCENARIOS.len() * 2);
    for seed in [L2_GAIN_DISCOVERY_SEED, L2_GAIN_VALIDATION_SEED] {
        for scenario in L2_GAIN_SCENARIOS {
            let negative_reward_required = if scenario == "stable" { 5 } else { 3 };
            reports.push(run_l1_l2_factorial_benchmark(
                scenario,
                runs,
                episodes,
                steps_per_episode,
                seed,
                0.35,
                3,
                negative_reward_required,
            )?);
        }
    }
    let assessment = assess_l2_gain_matrix(&reports)?;
    let kernel_runs = runs as u64 * L2_GAIN_SCENARIOS.len() as u64 * 2 * 4;
    Ok(L2GainMatrixReport {
        schema_version: 1,
        design: "paired_l1_l2_factorial_discovery_validation_v1".into(),
        runs_per_scenario: runs,
        episodes,
        steps_per_episode,
        scenarios: L2_GAIN_SCENARIOS,
        discovery_seed: L2_GAIN_DISCOVERY_SEED,
        validation_seed: L2_GAIN_VALIDATION_SEED,
        kernel_runs,
        decision_steps: kernel_runs * episodes as u64 * steps_per_episode as u64,
        reports,
        assessment,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn gain_matrix_has_fixed_independent_design_and_replay() {
        let first = run_l2_gain_matrix(2, 8, 8).unwrap();
        let second = run_l2_gain_matrix(2, 8, 8).unwrap();
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        assert_eq!(first.schema_version, 1);
        assert_eq!(first.reports.len(), 8);
        assert_eq!(first.kernel_runs, 64);
        assert_eq!(first.decision_steps, 4_096);
        assert_ne!(first.discovery_seed, first.validation_seed);
        assert!(first.discovery_seed + first.runs_per_scenario as u64 <= first.validation_seed);
        for report in &first.reports {
            assert_eq!(report.runs, 2);
            assert_eq!(report.arm_order, FactorialArm::ALL);
            for pair in &report.paired_runs {
                for (arm, arm_report) in FactorialArm::ALL.iter().zip(&pair.arms) {
                    assert!(arm_report.mean_reward.is_finite());
                    assert!(arm_report.mean_adjusted_reward.is_finite());
                    if !arm.l1_enabled() {
                        assert_eq!(arm_report.patches_accepted, 0);
                        assert_eq!(arm_report.l1_rollback_count, 0);
                    }
                    if !arm.l2_enabled() {
                        assert_eq!(arm_report.l2_proposals, 0);
                        assert_eq!(arm_report.l2_commits, 0);
                        assert_eq!(arm_report.committed_policies_applied, 0);
                        assert_eq!(arm_report.l2_rollback_count, 0);
                    }
                }
            }
        }
        let mut missing_primary = first.reports.clone();
        missing_primary.retain(|report| {
            !(report.scenario == "regime_shift" && report.seed == L2_GAIN_VALIDATION_SEED)
        });
        assert!(assess_l2_gain_matrix(&missing_primary).is_err());
        let mut duplicate_primary = first.reports.clone();
        duplicate_primary[6] = duplicate_primary[2].clone();
        assert!(assess_l2_gain_matrix(&duplicate_primary).is_err());
        let mut missing_seed = first.reports.clone();
        missing_seed[2].paired_runs.pop();
        assert!(assess_l2_gain_matrix(&missing_seed).is_err());
        assert!(run_l2_gain_matrix(1, 8, 8).is_err());
        assert!(run_l2_gain_matrix(2, 0, 8).is_err());
        assert!(run_l2_gain_matrix(2, 8, 0).is_err());
    }

    #[test]
    fn gain_verdict_requires_execution_discovery_validation_and_safety() {
        let mut matrix = run_l2_gain_matrix(2, 8, 8).unwrap();
        for report in &mut matrix.reports {
            for pair in &mut report.paired_runs {
                pair.arms[FactorialArm::L2.index()].committed_policies_applied = 1;
            }
            if report.scenario == "regime_shift" {
                report.l2_effect.reward = PairedMeanDifference {
                    mean: 0.02,
                    ci95_low: Some(0.01),
                    ci95_high: Some(0.03),
                };
            }
            if report.scenario == "stable" {
                report.l2_effect.reward = PairedMeanDifference {
                    mean: 0.0,
                    ci95_low: Some(-0.005),
                    ci95_high: Some(0.005),
                };
            }
        }
        let assessment = assess_l2_gain_matrix(&matrix.reports).unwrap();
        assert!(assessment.mechanism_precondition_met);
        assert!(assessment.reproducible_gain);

        let mut no_execution = matrix.clone();
        for report in &mut no_execution.reports {
            if report.scenario == "regime_shift" && report.seed == L2_GAIN_VALIDATION_SEED {
                for pair in &mut report.paired_runs {
                    pair.arms[FactorialArm::L2.index()].committed_policies_applied = 0;
                }
            }
        }
        let assessment = assess_l2_gain_matrix(&no_execution.reports).unwrap();
        assert!(!assessment.mechanism_precondition_met);
        assert!(!assessment.reproducible_gain);
        assert!(assessment.conclusion.contains("No executed"));

        let mut failed_replication = matrix.clone();
        let validation = failed_replication
            .reports
            .iter_mut()
            .find(|report| {
                report.scenario == "regime_shift" && report.seed == L2_GAIN_VALIDATION_SEED
            })
            .unwrap();
        validation.l2_effect.reward.ci95_low = Some(-0.01);
        let assessment = assess_l2_gain_matrix(&failed_replication.reports).unwrap();
        assert!(!assessment.validation_reproduced);
        assert!(!assessment.reproducible_gain);
        assert!(assessment.conclusion.contains("did not reproduce"));

        let mut unsafe_stable = matrix;
        let stable = unsafe_stable
            .reports
            .iter_mut()
            .find(|report| report.scenario == "stable" && report.seed == L2_GAIN_DISCOVERY_SEED)
            .unwrap();
        stable.l2_effect.reward.ci95_low = Some(-0.011);
        let assessment = assess_l2_gain_matrix(&unsafe_stable.reports).unwrap();
        assert!(!assessment.discovery_safety_passed);
        assert!(!assessment.reproducible_gain);
    }

    #[test]
    fn preregistered_gain_defaults_are_large_and_unchanged() {
        assert_eq!(L2_GAIN_DEFAULT_RUNS, 30);
        assert_eq!(L2_GAIN_DEFAULT_EPISODES, 12);
        assert_eq!(L2_GAIN_DEFAULT_STEPS, 12);
        assert_eq!(L2_GAIN_DISCOVERY_SEED, 20261115);
        assert_eq!(L2_GAIN_VALIDATION_SEED, 20270001);
        assert_eq!(L2_GAIN_SCENARIOS.len(), 4);
        let kernel_runs = L2_GAIN_DEFAULT_RUNS as u64 * 4 * 2 * 4;
        assert_eq!(kernel_runs, 960);
        assert_eq!(
            kernel_runs * L2_GAIN_DEFAULT_EPISODES as u64 * L2_GAIN_DEFAULT_STEPS as u64,
            138_240
        );
    }

    fn test_path(label: &str) -> std::path::PathBuf {
        let dir = temp_dir().join(format!(
            "omega-task-{}-{}-{}",
            std::process::id(),
            label,
            crate::new_uuid()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("state.json")
    }

    #[test]
    fn factorial_benchmark_is_deterministic_and_separates_gates() {
        let run =
            || run_l1_l2_factorial_benchmark("regime_shift", 2, 8, 8, 73, 0.35, 3, 3).unwrap();
        let first = run();
        let second = run();
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        assert_eq!(first.arm_order, FactorialArm::ALL);
        for pair in &first.paired_runs {
            for (arm, report) in FactorialArm::ALL.iter().zip(&pair.arms) {
                if !arm.l1_enabled() {
                    assert_eq!(report.patches_accepted, 0);
                    assert_eq!(report.l1_rollback_count, 0);
                }
                if !arm.l2_enabled() {
                    assert_eq!(report.l2_proposals, 0);
                    assert_eq!(report.l2_commits, 0);
                    assert_eq!(report.committed_policies_applied, 0);
                    assert_eq!(report.l2_rollback_count, 0);
                    assert_eq!(report.l1_blocked_by_l2_count, 0);
                }
                assert!(report.mean_adjusted_reward.is_finite());
            }
            let values = [
                pair.arms[0].mean_reward,
                pair.arms[1].mean_reward,
                pair.arms[2].mean_reward,
                pair.arms[3].mean_reward,
            ];
            assert_eq!(pair.l1_reward_delta, values[1] - values[0]);
            assert_eq!(pair.l2_reward_delta, values[2] - values[0]);
            assert_eq!(pair.joint_reward_delta, values[3] - values[0]);
            assert_eq!(
                pair.interaction_reward_delta,
                values[3] - values[2] - values[1] + values[0]
            );
        }
        assert!(first
            .paired_runs
            .iter()
            .any(|pair| pair.arms[1].patches_accepted > 0));
        assert!(first
            .paired_runs
            .iter()
            .any(|pair| pair.arms[3].l1_blocked_by_l2_count > 0));
    }

    #[test]
    fn factorial_effect_uses_four_arm_interaction_contrast() {
        let report =
            run_l1_l2_factorial_benchmark("nonstationary", 3, 8, 8, 20261115, 0.35, 3, 3).unwrap();
        for (effect, selector) in [
            (&report.l1_effect, 0),
            (&report.l2_effect, 1),
            (&report.joint_effect, 2),
            (&report.interaction_effect, 3),
        ] {
            let differences = factorial_differences(&report.paired_runs, |arm| arm.mean_reward);
            assert_eq!(effect.reward, paired_difference(&differences[selector]));
        }
        assert!(run_l1_l2_factorial_benchmark("invalid", 1, 1, 1, 1, 0.35, 3, 3).is_err());
    }

    #[test]
    fn score_aggregation_and_json_ignore_hash_map_iteration_order() {
        let path = test_path("aggregate-order");
        let mut kernel = Kernel::with_bandit_ucb(&path);
        kernel.autosave = false;
        kernel.auto_checkpoint_on_patch = false;
        kernel.dump_trajectory = false;
        let mut left =
            run_bandit_task_seeded_scenario(&mut kernel, 2, 4, 73, "regime_shift", None).unwrap();
        let mut right = left.clone();
        left.final_score_breakdown = HashMap::from_iter([
            ("a".into(), 1.0e16),
            ("b".into(), 1.0),
            ("c".into(), -1.0e16),
        ]);
        right.final_score_breakdown = HashMap::from_iter([
            ("c".into(), -1.0e16),
            ("b".into(), 1.0),
            ("a".into(), 1.0e16),
        ]);
        assert_eq!(
            serde_json::to_vec(&left).unwrap(),
            serde_json::to_vec(&right).unwrap()
        );

        let mut reverse = right.clone();
        reverse.seed = left.seed.wrapping_add(1);
        let forward = aggregate(&[left.clone(), right.clone()]);
        let backward = aggregate(&[reverse, left]);
        assert_eq!(
            forward.mean_final_score_breakdown["a"],
            backward.mean_final_score_breakdown["a"]
        );
        assert_eq!(
            forward.mean_final_score_breakdown["b"],
            backward.mean_final_score_breakdown["b"]
        );
        assert_eq!(
            forward.mean_final_score_breakdown["c"],
            backward.mean_final_score_breakdown["c"]
        );
        assert_eq!(
            serde_json::to_vec(&forward.mean_final_score_breakdown).unwrap(),
            serde_json::to_vec(&backward.mean_final_score_breakdown).unwrap()
        );
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn bandit_runs() {
        let path = test_path("bandit");
        let mut k = Kernel::with_state_path(&path);
        k.autosave = false;
        let report = run_bandit_task_seeded(&mut k, 4, 3, 42).unwrap();
        assert_eq!(report.episodes, 4);
        assert!(report.mean_reward >= 0.0);
        assert!(report.strategy_count >= 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn reward_feeds_strategy_memory() {
        let path = test_path("reward");
        let mut k = Kernel::with_state_path(&path);
        k.autosave = false;
        k.auto_checkpoint_on_patch = false;
        let before = k.inner.l0.memory.strategy.retrieve("", 20).len();
        let report = run_bandit_task_seeded(&mut k, 3, 4, 7).unwrap();
        let after = k.inner.l0.memory.strategy.retrieve("", 20);
        assert!(after.len() >= before);
        assert!(after.iter().any(|s| s.success_count + s.fail_count > 0));
        assert_eq!(report.strategy_count, after.len());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn seeded_bandit_is_deterministic() {
        let p1 = test_path("deterministic-a");
        let p2 = test_path("deterministic-b");
        let mut k1 = Kernel::with_state_path(&p1);
        let mut k2 = Kernel::with_state_path(&p2);
        k1.autosave = false;
        k2.autosave = false;
        k1.auto_checkpoint_on_patch = false;
        k2.auto_checkpoint_on_patch = false;
        let r1 = run_bandit_task_seeded(&mut k1, 5, 3, 12345).unwrap();
        let r2 = run_bandit_task_seeded(&mut k2, 5, 3, 12345).unwrap();
        assert_eq!(r1.mean_reward, r2.mean_reward);
        assert_eq!(r1.best_arm_rate, r2.best_arm_rate);
        assert_eq!(r1.strategy_count, r2.strategy_count);
        let _ = std::fs::remove_file(&p1);
        let _ = std::fs::remove_file(&p2);
    }

    #[test]
    fn bandit_reports_strategy_decay_metrics() {
        let path = test_path("decay");
        let mut k = Kernel::with_state_path(&path);
        k.autosave = false;
        k.auto_checkpoint_on_patch = false;
        // Longer run so some strategies go idle
        let report = run_bandit_task_seeded(&mut k, 6, 5, 99).unwrap();
        assert!(report.strategy_count >= 1);
        assert!(report.mean_strategy_weight > 0.0);
        // max_idle_steps should be tracked (may be 0 if all actions stay hot)
        let _ = report.max_idle_steps;
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn regime_shift_benchmark_exposes_recovery_and_residual() {
        let report = run_l2_benchmark_scenario("regime_shift", 2, 8, 8, 73).unwrap();
        assert_eq!(report.scenario, "regime_shift");
        for branch in [&report.baseline_l2_disabled, &report.treatment_l2_enabled] {
            assert!(branch.recovery_rate >= 0.0 && branch.recovery_rate <= 1.0);
            assert!(branch.mean_final_dayan_residual.is_finite());
            assert!(branch.mean_l1_rollback_count >= 0.0);
            assert!(branch.mean_dwell_hold_count >= 0.0);
        }
        assert!(report.treatment_l2_enabled.mean_dwell_hold_count > 0.0);
    }

    #[test]
    fn diagnostic_statistics_and_paired_replay_are_deterministic() {
        let ranks = ranks(&[0.2, 0.1, 0.2, 0.4]);
        assert_eq!(ranks, vec![2.5, 1.0, 2.5, 4.0]);
        assert!(pearson(&[1.0, 1.0], &[0.0, 1.0]).is_none());
        let constant = correlation(&[(0.0, 0.0); 8]);
        assert_eq!(constant.pearson, None);
        assert_eq!(constant.spearman, None);
        let settings = L2ExperimentSettings {
            spectral_control: true,
            purity_prior: true,
            reverse_brake: true,
            history_discount: true,
        };
        let first = run_l2_benchmark_scenario_with_gate(
            "regime_shift",
            2,
            8,
            8,
            20261115,
            0.35,
            3,
            3,
            Some(settings.clone()),
        )
        .unwrap();
        let second = run_l2_benchmark_scenario_with_gate(
            "regime_shift",
            2,
            8,
            8,
            20261115,
            0.35,
            3,
            3,
            Some(settings),
        )
        .unwrap();
        let json = |report: &L2BenchmarkReport| serde_json::to_value(report).unwrap();
        let first_json = json(&first);
        let second_json = json(&second);
        if first_json != second_json {
            fn first_difference<'a>(
                path: &str,
                left: &'a serde_json::Value,
                right: &'a serde_json::Value,
            ) -> Option<(String, &'a serde_json::Value, &'a serde_json::Value)> {
                match (left, right) {
                    (serde_json::Value::Object(left), serde_json::Value::Object(right)) => {
                        let mut keys: Vec<_> = left.keys().chain(right.keys()).collect();
                        keys.sort_unstable();
                        keys.dedup();
                        keys.into_iter().find_map(|key| {
                            first_difference(
                                &format!("{path}.{key}"),
                                left.get(key).unwrap_or(&serde_json::Value::Null),
                                right.get(key).unwrap_or(&serde_json::Value::Null),
                            )
                        })
                    }
                    (serde_json::Value::Array(left), serde_json::Value::Array(right)) => {
                        (0..left.len().max(right.len())).find_map(|index| {
                            first_difference(
                                &format!("{path}[{index}]"),
                                left.get(index).unwrap_or(&serde_json::Value::Null),
                                right.get(index).unwrap_or(&serde_json::Value::Null),
                            )
                        })
                    }
                    _ if left == right => None,
                    _ => Some((path.to_string(), left, right)),
                }
            }
            panic!(
                "replay mismatch: {:?}",
                first_difference("$", &first_json, &second_json)
            );
        }
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        for (left, right) in first.paired_runs.iter().zip(&second.paired_runs) {
            for (a, b) in [
                (&left.baseline, &right.baseline),
                (&left.treatment, &right.treatment),
            ] {
                assert_eq!(a.mean_reward, b.mean_reward);
                assert_eq!(a.best_arm_rate, b.best_arm_rate);
                assert_eq!(a.action_switches, b.action_switches);
                assert_eq!(a.patches_accepted, b.patches_accepted);
                assert_eq!(a.diagnostics, b.diagnostics);
            }
        }
        for run in first.paired_runs.iter().map(|run| &run.treatment) {
            let pipeline = run.pipeline.as_ref().unwrap();
            assert_eq!(pipeline.proposals, run.l2_commits as u64);
            for sample in run.diagnostics.as_ref().unwrap() {
                assert_eq!(sample.dry_run_treatment_arm, Some(sample.selected_arm));
            }
        }
        let run = &first.paired_runs[0].treatment;
        assert_eq!(run.pipeline.as_ref().unwrap().observations, 64);
        assert_eq!(run.diagnostics.as_ref().unwrap().len(), 64);
        assert!(run.signals.as_ref().unwrap().bias_to_next_reward.samples > 0);
    }

    #[test]
    fn diagnostics_do_not_change_the_paired_run() {
        let settings = L2ExperimentSettings::default();
        let plain = run_l2_benchmark_scenario_with_gate(
            "nonstationary",
            3,
            8,
            8,
            20261115,
            0.35,
            3,
            3,
            None,
        )
        .unwrap();
        let observed = run_l2_benchmark_scenario_with_gate(
            "nonstationary",
            3,
            8,
            8,
            20261115,
            0.35,
            3,
            3,
            Some(settings),
        )
        .unwrap();
        let without_diagnostics = |report: &L2BenchmarkReport| {
            let mut value = serde_json::to_value(report).unwrap();
            for pair in value["paired_runs"].as_array_mut().unwrap() {
                for branch in ["baseline", "treatment"] {
                    let branch = &mut pair[branch];
                    for field in ["pipeline", "diagnostics", "signals", "experiment"] {
                        branch.as_object_mut().unwrap().remove(field);
                    }
                }
            }
            value
        };
        assert_eq!(without_diagnostics(&plain), without_diagnostics(&observed));
    }

    #[test]
    fn l2_benchmark_is_paired_and_reports_intervention_limits() {
        let report = run_l2_benchmark(1, 8, 8, 42).unwrap();
        assert_eq!(report.runs, 1);
        assert_eq!(report.baseline_l2_disabled.mean_l2_nudges, 0.0);
        assert_eq!(report.baseline_l2_disabled.mean_l2_proposals, 0.0);
        assert_eq!(report.baseline_l2_disabled.mean_l2_commits, 0.0);
        assert_eq!(
            report.baseline_l2_disabled.mean_committed_policies_applied,
            0.0
        );
        assert!(report.treatment_l2_enabled.mean_final_alignment_score > 0.0);
        assert!(report.treatment_l2_enabled.mean_dwell_hold_count > 0.0);
        // A proposal/cache hit is not evidence that L2 influenced L0. Any
        // actual commit must be attributed to its immutable applied snapshot.
        assert_eq!(report.paired_reward_difference.ci95_low, None);
        assert_eq!(report.paired_reward_difference.ci95_high, None);
        assert_eq!(report.paired_runs.len(), 1);
        let paired = &report.paired_runs[0];
        let treatment_influenced_l0 =
            paired.treatment.committed_policies_applied > 0 || paired.treatment.l2_nudges > 0;
        assert_eq!(
            report.l0_influenced_runs,
            u32::from(treatment_influenced_l0)
        );
        assert_eq!(paired.seed, 42);
        assert_eq!(
            paired.delta_mean_reward,
            paired.treatment.mean_reward - paired.baseline.mean_reward
        );
        assert_eq!(
            paired.delta_best_arm_rate,
            paired.treatment.best_arm_rate - paired.baseline.best_arm_rate
        );
        assert_eq!(paired.treatment_influenced_l0, treatment_influenced_l0);
        for metrics in &paired.treatment.expert_metrics {
            assert_eq!(metrics.committed, metrics.applied + metrics.not_applied);
            if metrics.applied == 0 {
                assert_eq!(metrics.reward_sum, 0.0);
            }
        }
        assert!(
            report.treatment_l2_enabled.mean_committed_policies_applied
                <= report.treatment_l2_enabled.mean_l2_commits
        );
    }
}
