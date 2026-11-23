//! Minimal task environment: nonstationary 2-armed bandit.
//! External reward is written back into strategy memory so the task loop is closed.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::api::Kernel;
use crate::error::Result;
use crate::l0::decision::DecisionMaker;
use hashbrown::HashMap;
use serde::Serialize;
use std::env::temp_dir;

#[derive(Debug, Clone, Serialize)]
pub struct BanditReport {
    pub scenario: String,
    pub episodes: u32,
    pub steps_per_episode: u32,
    pub mean_reward: f64,
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
    pub final_score_breakdown: HashMap<String, f64>,
    pub final_alignment_score: f64,
    pub recovery_steps: Option<u32>,
    pub l1_rollback_count: u64,
    pub l2_rollback_count: u64,
    pub dayan_converged: bool,
    pub final_dayan_residual: f64,
    pub dwell_hold_count: u64,
    pub margin_hold_count: u64,
    pub active_steps: u64,
    pub committed_policies_applied: u64,
    pub lyapunov_vetoes: u64,
    pub meta_cache_hits: u64,
    pub recovery_latencies: Vec<u32>,
    pub warm_recovery_latencies: Vec<u32>,
    pub mean_residual_improvement: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct L2BenchmarkAggregate {
    pub runs: u32,
    pub mean_reward: f64,
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
    pub mean_final_score_breakdown: HashMap<String, f64>,
    pub mean_final_alignment_score: f64,
    pub mean_recovery_steps: Option<f64>,
    pub recovery_rate: f64,
    pub mean_l1_rollback_count: f64,
    pub mean_l2_rollback_count: f64,
    pub dayan_convergence_rate: f64,
    pub mean_final_dayan_residual: f64,
    pub mean_dwell_hold_count: f64,
    pub mean_margin_hold_count: f64,
    pub mean_active_steps: f64,
    pub mean_committed_policies_applied: f64,
    pub mean_lyapunov_vetoes: f64,
    pub mean_meta_cache_hits: f64,
    pub recovery_event_rate: f64,
    pub mean_warm_recovery_steps: Option<f64>,
    pub mean_residual_improvement: f64,
}

#[derive(Debug, Clone, Copy, Serialize)]
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
    pub l0_influenced_runs: u32,
    /// Per-seed evidence for diagnosing differences that occur without a commit.
    pub paired_runs: Vec<L2PairedRun>,
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
    run_bandit_task_seeded_scenario(kernel, episodes, steps_per_episode, seed, "nonstationary")
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
) -> Result<BanditReport>
where
    D: DecisionMaker,
{
    let mut rng = Rng::new(seed);
    let mut total_reward = 0.0;
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
    let mut pending_commit: Option<(String, usize)> = None;
    let mut max_fixed_point_dwell = 0u64;
    let mut consequence_sum = 0.0;
    let mut early_err = None;
    let mut final_score_breakdown = HashMap::new();
    let mut recovery_steps = None;
    let mut recovery_streak = 0;
    let mut l1_rollback_count = 0;
    let mut l2_rollback_count = 0;
    let mut final_dayan_residual = 1.0;
    let mut dwell_hold_count = 0;
    let mut margin_hold_count = 0;
    let mut active_steps = 0;
    let mut committed_policies_applied = 0;
    let mut lyapunov_vetoes = 0;
    let mut meta_cache_hits = 0;
    let mut recovery_latencies = Vec::new();
    let mut warm_recovery_latencies = Vec::new();
    let mut recovery_started_at = None;
    let mut recovery_warm = false;
    let mut recovery_used_commit = false;
    let mut last_warm_commit: Option<crate::types::L2Policy> = None;
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
            let logged_policy = r
                .trajectory
                .actions
                .first()
                .and_then(|action| action.params.get("l2_policy"))
                .and_then(|value| {
                    serde_json::from_value::<crate::types::L2Policy>(value.clone()).ok()
                })
                .unwrap_or_default();
            debug_assert_eq!(applied, logged_policy);
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
            active_steps += u64::from(r.l2.gate_state == crate::l2::control::L2GateState::Active);
            committed_policies_applied += u64::from(applied.committed);
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
            if let Some((before_action, before_arm)) = pending_commit.take() {
                if action != before_action {
                    post_commit_action_changes += 1;
                }
                if arm != before_arm {
                    post_commit_arm_changes += 1;
                }
            }
            if r.l2.modulation_committed {
                pending_commit = Some((action.clone(), arm));
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

    Ok(BanditReport {
        scenario: scenario.into(),
        episodes,
        steps_per_episode,
        mean_reward: if total_choices == 0 {
            0.0
        } else {
            total_reward / total_choices as f64
        },
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
        lyapunov_vetoes,
        meta_cache_hits,
        recovery_latencies,
        warm_recovery_latencies,
        mean_residual_improvement: residual_improvement / total_choices.max(1) as f64,
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
            mean_l2_rollback_count: 0.0,
            dayan_convergence_rate: 0.0,
            mean_final_dayan_residual: 0.0,
            mean_dwell_hold_count: 0.0,
            mean_margin_hold_count: 0.0,
            mean_active_steps: 0.0,
            mean_committed_policies_applied: 0.0,
            mean_lyapunov_vetoes: 0.0,
            mean_meta_cache_hits: 0.0,
            recovery_event_rate: 0.0,
            mean_warm_recovery_steps: None,
            mean_residual_improvement: 0.0,
        };
    }
    let n = reports.len() as f64;
    let mean = |f: fn(&BanditReport) -> f64| reports.iter().map(f).sum::<f64>() / n;
    L2BenchmarkAggregate {
        runs,
        mean_reward: mean(|r| r.mean_reward),
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
            let mut out = HashMap::new();
            for report in reports {
                for (key, value) in &report.final_score_breakdown {
                    *out.entry(key.clone()).or_insert(0.0) += value / n;
                }
            }
            out
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
        mean_l2_rollback_count: mean(|r| r.l2_rollback_count as f64),
        dayan_convergence_rate: reports.iter().filter(|r| r.dayan_converged).count() as f64 / n,
        mean_final_dayan_residual: mean(|r| r.final_dayan_residual),
        mean_dwell_hold_count: mean(|r| r.dwell_hold_count as f64),
        mean_margin_hold_count: mean(|r| r.margin_hold_count as f64),
        mean_active_steps: mean(|r| r.active_steps as f64),
        mean_committed_policies_applied: mean(|r| r.committed_policies_applied as f64),
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
        )?;
        cleanup_benchmark_files(&base_path);
        let treatment_report = run_bandit_task_seeded_scenario(
            &mut l2,
            episodes,
            steps_per_episode,
            run_seed,
            scenario,
        )?;
        cleanup_benchmark_files(&treatment_path);
        paired_runs.push(L2PairedRun {
            seed: run_seed,
            delta_mean_reward: treatment_report.mean_reward - base_report.mean_reward,
            delta_best_arm_rate: treatment_report.best_arm_rate - base_report.best_arm_rate,
            delta_error_delta: (treatment_report.final_avg_error
                - treatment_report.initial_avg_error)
                - (base_report.final_avg_error - base_report.initial_avg_error),
            treatment_influenced_l0: treatment_report.l2_commits > 0
                || treatment_report.committed_policies_applied > 0
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
        l0_influenced_runs,
        baseline_l2_disabled: baseline,
        treatment_l2_enabled: treatment,
        paired_runs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

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
        assert_eq!(report.l0_influenced_runs, 0);
        assert_eq!(report.paired_runs.len(), 1);
        let paired = &report.paired_runs[0];
        assert_eq!(paired.seed, 42);
        assert_eq!(
            paired.delta_mean_reward,
            paired.treatment.mean_reward - paired.baseline.mean_reward
        );
        assert_eq!(
            paired.delta_best_arm_rate,
            paired.treatment.best_arm_rate - paired.baseline.best_arm_rate
        );
        assert!(!paired.treatment_influenced_l0);
        assert!(
            report.treatment_l2_enabled.mean_committed_policies_applied
                <= report.treatment_l2_enabled.mean_l2_commits
        );
    }
}
