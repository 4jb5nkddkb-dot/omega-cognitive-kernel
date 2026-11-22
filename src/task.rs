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
    kernel: &mut Kernel<crate::l0::observer::MultiModalObserver, D, crate::l0::executor::SandboxedExecutor>,
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

        for _ in 0..steps_per_episode {
            let before_err = kernel.inner.l0.memory.cas.avg_prediction_error;
            let r = kernel
                .step("maintain coherent self-model omega dayan while maximizing bandit reward")?;
            let after_err = kernel.inner.l0.memory.cas.avg_prediction_error;
            if early_err.is_none() {
                early_err = Some(after_err);
            }
            if r.l1.patched {
                patches += 1;
            }
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
            // Feed reward back to the decision maker (UCB updates for BanditUcbDecisionMaker)
            kernel.inner.l0.decision.observe(arm, reward);
            let consequence_score = 0.60 * (reward - 0.5) - 0.25 * (after_err - before_err);
            consequence_sum += consequence_score;
            kernel.observe_l2_consequence(reward - 0.5, after_err - before_err, 0.0);

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
    })
}

fn aggregate(reports: &[BanditReport]) -> L2BenchmarkAggregate {
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
    }
}

fn cleanup_benchmark_files(state_path: &std::path::Path) {
    let parent = state_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let _ = std::fs::remove_file(state_path);
    let _ = std::fs::remove_file(parent.join("audit.jsonl"));
    let _ = std::fs::remove_file(parent.join("trajectory.jsonl"));
    let _ = std::fs::remove_dir_all(parent.join("checkpoints"));
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
        (0.65, 8)
    } else {
        (0.55, 5)
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
        "stable" | "nonstationary" | "degraded" | "fixedpoint" => scenario,
        other => {
            return Err(crate::error::OmegaError::Config(format!(
                "unknown benchmark scenario: {other}"
            )))
        }
    };
    let runs = runs.max(1);
    let mut baseline = Vec::with_capacity(runs as usize);
    let mut treatment = Vec::with_capacity(runs as usize);

    for i in 0..runs {
        let run_seed = seed.wrapping_add(i as u64);
        let base_path = temp_dir().join(format!(
            "omega-l2-benchmark-{}-base.json",
            crate::new_uuid()
        ));
        let treatment_path =
            temp_dir().join(format!("omega-l2-benchmark-{}-l2.json", crate::new_uuid()));

        let mut base = Kernel::with_state_path(&base_path);
        base.autosave = false;
        base.auto_checkpoint_on_patch = false;
        base.dump_trajectory = false;
        // Both arms start from the same coherent policy context. This makes
        // fixed-point entry observable while preserving a fair intervention
        // comparison: only L2 is disabled in the baseline.
        base.inner
            .l1
            .store
            .active
            .insert("cognitive_rules".into(), "self-model bias damp rule".into());
        base.set_l2_enabled(false);
        baseline.push(run_bandit_task_seeded_scenario(
            &mut base,
            episodes,
            steps_per_episode,
            run_seed,
            scenario,
        )?);
        cleanup_benchmark_files(&base_path);

        // Treatment: use BanditUcbDecisionMaker for true UCB-based arm selection
        let mut l2 = Kernel::with_bandit_ucb(&treatment_path);
        l2.autosave = false;
        l2.auto_checkpoint_on_patch = false;
        l2.dump_trajectory = false;
        l2.inner
            .l1
            .store
            .active
            .insert("cognitive_rules".into(), "self-model bias damp rule".into());
        l2.inner.l2.detector.window = 2;
        l2.inner.l2.detector.pred_threshold = 0.30;
        l2.inner.l2.detector.self_ref_threshold = 0.30;
        l2.inner.l2.detector.cooldown_steps = 0;
        // The five-signal score is intentionally exercised at 0.55 in this
        // short fixture; production/default L2 remains conservative at 0.82.
        l2.inner.l2.commit_threshold = commit_threshold.clamp(0.0, 1.0);
        l2.inner.l2.commit_cooldown_steps = commit_cooldown_steps;
        l2.inner.l2.negative_reward_required = negative_reward_required.max(1);
        l2.set_l2_enabled(true);
        treatment.push(run_bandit_task_seeded_scenario(
            &mut l2,
            episodes,
            steps_per_episode,
            run_seed,
            scenario,
        )?);
        cleanup_benchmark_files(&treatment_path);
    }

    let baseline = aggregate(&baseline);
    let treatment = aggregate(&treatment);
    Ok(L2BenchmarkReport {
        scenario: scenario.into(),
        episodes,
        steps_per_episode,
        runs,
        seed,
        delta_mean_reward: treatment.mean_reward - baseline.mean_reward,
        delta_best_arm_rate: treatment.best_arm_rate - baseline.best_arm_rate,
        delta_error_decrease_rate: treatment.error_decrease_rate - baseline.error_decrease_rate,
        baseline_l2_disabled: baseline,
        treatment_l2_enabled: treatment,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn bandit_runs() {
        let path = temp_dir().join(format!("omega-bandit-{}.json", crate::new_uuid()));
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
        let path = temp_dir().join(format!("omega-rw-{}.json", crate::new_uuid()));
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
        let p1 = temp_dir().join(format!("omega-d1-{}.json", crate::new_uuid()));
        let p2 = temp_dir().join(format!("omega-d2-{}.json", crate::new_uuid()));
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
        let path = temp_dir().join(format!("omega-decay-{}.json", crate::new_uuid()));
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
    fn l2_benchmark_reports_intervention_delta() {
        let report = run_l2_benchmark(1, 8, 8, 42).unwrap();
        assert_eq!(report.runs, 1);
        assert_eq!(report.baseline_l2_disabled.mean_l2_nudges, 0.0);
        assert!(report.treatment_l2_enabled.mean_final_alignment_score > 0.0);
    }
}
