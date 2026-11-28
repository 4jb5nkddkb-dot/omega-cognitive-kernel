#[allow(unused_imports)]
use omega_cognitive_kernel::prelude::*;

use clap::{Parser, Subcommand, ValueEnum};
use omega_cognitive_kernel::api::{
    replay_trajectory_file, ClosedLoopKernel, ClosedLoopSession, ExternalOutcome, Kernel,
    StepTelemetry, LoopBenchmarkScenario, run_closed_loop_matrix_benchmark,
};
use omega_cognitive_kernel::audit::AuditLog;
use omega_cognitive_kernel::config::{InterventionPolicy, KernelConfig, LoopPreset};
#[cfg(feature = "toml")]
use omega_cognitive_kernel::config_toml;
use omega_cognitive_kernel::task::{
    run_bandit_task, run_l1_l2_factorial_benchmark, run_l2_benchmark_scenario_with_gate,
    run_l2_gain_matrix, L2ExperimentSettings, L2_GAIN_DEFAULT_EPISODES, L2_GAIN_DEFAULT_RUNS,
    L2_GAIN_DEFAULT_STEPS, L2_GAIN_DISCOVERY_SEED,
};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Copy, Clone, Debug, ValueEnum)]
enum CliLoopBenchmarkScenario {
    Steady,
    Shift,
    Noisy,
}

impl From<CliLoopBenchmarkScenario> for LoopBenchmarkScenario {
    fn from(scenario: CliLoopBenchmarkScenario) -> Self {
        match scenario {
            CliLoopBenchmarkScenario::Steady => Self::Steady,
            CliLoopBenchmarkScenario::Shift => Self::Shift,
            CliLoopBenchmarkScenario::Noisy => Self::Noisy,
        }
    }
}
#[derive(Copy, Clone, Debug, ValueEnum)]
enum CliLoopPreset {
    Deterministic,
    HotPatcher,
    MetaGuard,
    FastAdapt,
    SteadyPrior,
    ControlPlane,
    FullKernel,
}

impl From<CliLoopPreset> for LoopPreset {
    fn from(preset: CliLoopPreset) -> Self {
        match preset {
            CliLoopPreset::Deterministic => Self::Deterministic,
            CliLoopPreset::HotPatcher => Self::HotPatcher,
            CliLoopPreset::MetaGuard => Self::MetaGuard,
            CliLoopPreset::FastAdapt => Self::FastAdapt,
            CliLoopPreset::SteadyPrior => Self::SteadyPrior,
            CliLoopPreset::ControlPlane => Self::ControlPlane,
            CliLoopPreset::FullKernel => Self::FullKernel,
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum CliExternalOutcome {
    Improved,
    Degraded,
    Timeout,
}

impl From<CliExternalOutcome> for ExternalOutcome {
    fn from(outcome: CliExternalOutcome) -> Self {
        match outcome {
            CliExternalOutcome::Improved => Self::AcceptedAndImproved,
            CliExternalOutcome::Degraded => Self::Degraded,
            CliExternalOutcome::Timeout => Self::TimeoutOrSkipped,
        }
    }
}

#[derive(Parser)]
#[command(
    name = "omega-kernel",
    version,
    about = "Omega Cognitive Kernel — self-improving cognitive runtime (L0/L1/L2)"
)]
struct Cli {
    /// Path to omega.toml (optional). Applied after open; --state overrides config state_path.
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[arg(long, global = true)]
    state: Option<PathBuf>,
    /// Closed-loop layer preset (deterministic, hot-patcher, meta-guard, fast-adapt, steady-prior, control-plane, full-kernel).
    #[arg(long, global = true, value_enum)]
    preset: Option<CliLoopPreset>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        #[arg(
            long,
            default_value = "maintain coherent self-model while acting in world"
        )]
        goal: String,
        #[arg(long, default_value_t = 12)]
        steps: u32,
        #[arg(long)]
        no_save: bool,
    },
    /// Print kernel status (human text or --json).
    Status {
        /// Emit KernelStatus as JSON (for scripts / HTTP shims).
        #[arg(long)]
        json: bool,
    },
    Reset {
        #[arg(long)]
        yes: bool,
    },
    Demo {
        #[arg(long, default_value_t = 12)]
        steps: u32,
    },
    /// Show audit trail (tail) or operator summary panel.
    Audit {
        #[arg(long, default_value_t = 20)]
        tail: usize,
        /// Print rollup panel instead of raw events.
        #[arg(long)]
        summary: bool,
    },
    /// Offline replay of trajectory.jsonl (no world re-execution).
    Replay {
        /// Explicit trajectory path; default is trajectory.jsonl beside --state.
        #[arg(long)]
        path: Option<PathBuf>,
        #[arg(long, default_value_t = 20)]
        tail: usize,
    },
    Checkpoints,
    Checkpoint {
        #[arg(long, default_value = "manual")]
        label: String,
    },
    Rollback {
        id: String,
    },
    Task {
        #[arg(long, default_value_t = 6)]
        episodes: u32,
        #[arg(long, default_value_t = 4)]
        steps: u32,
    },
    /// Compare identical seeded runs with L2 interventions disabled/enabled.
    BenchmarkL2 {
        /// stable, nonstationary, degraded, fixedpoint, regime_shift, or cyclic_shift
        #[arg(long, default_value = "nonstationary")]
        scenario: String,
        #[arg(long, default_value_t = 5)]
        runs: u32,
        #[arg(long, default_value_t = 8)]
        episodes: u32,
        #[arg(long, default_value_t = 8)]
        steps: u32,
        #[arg(long, default_value_t = 20261115)]
        seed: u64,
        /// L2 commit score threshold used by the treatment arm.
        #[arg(long)]
        commit_threshold: Option<f64>,
        /// Cooldown steps after a committed modulation.
        #[arg(long)]
        commit_cooldown: Option<u64>,
        /// Consecutive negative rewards required for reward_change signal.
        #[arg(long)]
        negative_reward_required: Option<u32>,
        /// Opt-in spectral experiment; default benchmark remains the baseline.
        #[arg(long)]
        spectral_control: bool,
        #[arg(long, requires = "spectral_control")]
        no_purity_prior: bool,
        #[arg(long, requires = "spectral_control")]
        no_reverse_brake: bool,
        #[arg(long, requires = "spectral_control")]
        no_history_discount: bool,
        /// Include per-step diagnostics in each treatment run.
        #[arg(long)]
        diagnostics: bool,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Compare L0, L1-only, L2-only, and L1+L2 on the same seeded task.
    BenchmarkL1l2 {
        /// stable, nonstationary, degraded, fixedpoint, regime_shift, or cyclic_shift
        #[arg(long, default_value = "regime_shift")]
        scenario: String,
        #[arg(long, default_value_t = 5)]
        runs: u32,
        #[arg(long, default_value_t = 8)]
        episodes: u32,
        #[arg(long, default_value_t = 8)]
        steps: u32,
        #[arg(long, default_value_t = 20261115)]
        seed: u64,
        #[arg(long)]
        commit_threshold: Option<f64>,
        #[arg(long)]
        commit_cooldown: Option<u64>,
        #[arg(long)]
        negative_reward_required: Option<u32>,
        /// Emit machine-readable JSON with all four per-seed reports.
        #[arg(long)]
        json: bool,
    },
    /// Run the fixed four-scenario, discovery/validation L2 gain matrix.
    BenchmarkL2Gain {
        /// Paired seeds per scenario; must be at least two for intervals.
        #[arg(long, default_value_t = L2_GAIN_DEFAULT_RUNS)]
        runs: u32,
        #[arg(long, default_value_t = L2_GAIN_DEFAULT_EPISODES)]
        episodes: u32,
        #[arg(long, default_value_t = L2_GAIN_DEFAULT_STEPS)]
        steps: u32,
        /// Run a quick two-seed preflight; not effectiveness evidence.
        #[arg(long, conflicts_with_all = ["runs", "episodes", "steps"])]
        smoke: bool,
        /// Emit all eight factorial reports and the preregistered assessment.
        #[arg(long)]
        json: bool,
    },
    /// Operator nudge: set strategy weight (0.1–10).
    StrategyWeight {
        /// Strategy name (e.g. observe, explore).
        name: String,
        /// New weight in [0.1, 10].
        weight: f64,
    },
    /// Incremental audit poll (byte cursor). Prints new events and next offset.
    AuditPoll {
        /// Byte offset from a previous poll (0 = from start / resync).
        #[arg(long, default_value_t = 0)]
        offset: u64,
    },
    /// Run a single external-telemetry step for L0-off presets (hot-patcher, meta-guard, control-plane).
    StepExternal {
        /// Monotonically increasing step identifier.
        #[arg(long, default_value_t = 1)]
        step_id: u64,
        /// Observed prediction or policy error.
        #[arg(long)]
        error: f64,
        /// Observed action risk in [0, 1].
        #[arg(long, default_value_t = 0.0)]
        risk: f64,
        /// Outcome of the previous directive, if any.
        #[arg(long, value_enum)]
        outcome: Option<CliExternalOutcome>,
        /// Run L1 in shadow-only mode without mutating cooldown state.
        #[arg(long)]
        shadow: bool,
        /// Emit ControlDirective as JSON.
        #[arg(long)]
        json: bool,
        /// Do not persist updated external closed-loop state after stepping.
        #[arg(long)]
        no_save: bool,
    },
    /// Benchmark all 7 (L0, L1, L2) closed-loop presets under steady, shift, or noisy workloads.
    BenchmarkLoops {
        /// Workload scenario (steady, shift, noisy).
        #[arg(long, value_enum, default_value_t = CliLoopBenchmarkScenario::Shift)]
        scenario: CliLoopBenchmarkScenario,
        /// Steps to execute per preset.
        #[arg(long, default_value_t = 24)]
        steps: u32,
        /// Emit machine-readable JSON report.
        #[arg(long)]
        json: bool,
    },
}

fn state_path(cli: &Cli) -> Option<String> {
    cli.state.as_ref().map(|p| p.display().to_string())
}

fn load_intervention(cli: &Cli) -> InterventionPolicy {
    #[cfg(feature = "toml")]
    {
        if let Some(ref cfg_path) = cli.config {
            if let Ok(cfg) = config_toml::KernelConfig::load(cfg_path) {
                return cfg.intervention;
            }
        }
    }
    #[cfg(not(feature = "toml"))]
    let _ = cli;
    InterventionPolicy::default()
}
fn resolve_effective_config(cli: &Cli) -> anyhow::Result<KernelConfig> {
    #[allow(unused_mut)]
    let mut cfg = {
        #[cfg(feature = "toml")]
        {
            if let Some(ref cfg_path) = cli.config {
                config_toml::KernelConfig::load(cfg_path)?
            } else {
                KernelConfig::default()
            }
        }
        #[cfg(not(feature = "toml"))]
        {
            KernelConfig::default()
        }
    };
    if let Some(preset) = cli.preset {
        cfg = cfg.with_loop_preset(preset.into());
    }
    Ok(cfg)
}

fn open_kernel(cli: &Cli) -> anyhow::Result<Kernel> {
    #[cfg(feature = "toml")]
    if let Some(ref cfg_path) = cli.config {
        let mut cfg = config_toml::KernelConfig::load(cfg_path)?;
        if let Some(preset) = cli.preset {
            cfg = cfg.with_loop_preset(preset.into());
        }
        let state_path = cli
            .state
            .clone()
            .unwrap_or_else(|| cfg.resolved_state_path());
        let mut kernel = Kernel::open(Some(&state_path.to_string_lossy()))?;
        kernel.apply_config(&cfg);
        kernel.state_path = state_path;
        return Ok(kernel);
    }
    let mut kernel = Kernel::open(state_path(cli).as_deref())?;
    if let Some(preset) = cli.preset {
        let cfg = KernelConfig::default().with_loop_preset(preset.into());
        kernel.apply_config(&cfg);
    }
    Ok(kernel)
}

fn print_intervention_block(p: &InterventionPolicy) {
    let mode = match p.mode {
        omega_cognitive_kernel::InterventionMode::Exploration => "exploration",
        omega_cognitive_kernel::InterventionMode::Strict => "strict",
    };
    let max = match p.max_level {
        omega_cognitive_kernel::InterventionLevel::Silent => "silent",
        omega_cognitive_kernel::InterventionLevel::InFrame => "in_frame",
        omega_cognitive_kernel::InterventionLevel::SuggestAlternative => "suggest_alternative",
        omega_cognitive_kernel::InterventionLevel::RefuseToDecide => "refuse_to_decide",
    };
    let eff = match p.effective_max_level() {
        omega_cognitive_kernel::InterventionLevel::Silent => "silent",
        omega_cognitive_kernel::InterventionLevel::InFrame => "in_frame",
        omega_cognitive_kernel::InterventionLevel::SuggestAlternative => "suggest_alternative",
        omega_cognitive_kernel::InterventionLevel::RefuseToDecide => "refuse_to_decide",
    };
    println!("── intervention ──");
    println!("mode              : {mode}");
    println!("max_level         : {max}");
    println!("effective_max     : {eff}");
    println!("friction_threshold: {}", p.friction_threshold);
    println!("audit_escalations : {}", p.audit_escalations);
    println!("note              : path alternatives only if exploration && failures ≥ threshold");
}

fn print_audit_summary(log: &AuditLog, policy: &InterventionPolicy) -> anyhow::Result<()> {
    let s = log.summarize()?;
    println!("╔══════════════════════════════════════════════╗");
    println!("║           Omega audit panel                  ║");
    println!("╚══════════════════════════════════════════════╝");
    println!("log        : {}", log.path.display());
    println!("events     : {}", s.total_events);
    if let (Some(a), Some(b)) = (s.first_ts, s.last_ts) {
        println!("window     : {} → {}", a.to_rfc3339(), b.to_rfc3339());
    }
    print_intervention_block(policy);
    println!("── counts ──");
    println!("steps              : {}", s.steps);
    println!("patches accepted   : {}", s.patches_accepted);
    println!("patches rejected   : {}", s.patches_rejected);
    println!("checkpoints        : {}", s.checkpoints);
    println!("rollbacks          : {}", s.rollbacks);
    println!("task episodes      : {}", s.task_episodes);
    println!("tools allowed      : {}", s.tools_allowed);
    println!("tools denied       : {}", s.tools_denied);
    if let Some(a) = &s.last_action {
        println!("last action        : {a}");
    }
    if let Some(e) = s.last_avg_err {
        println!("last avg_err       : {e:.4}");
    }
    if let Some(d) = s.last_damping {
        println!("last damping       : {d:.4}");
    }
    if !s.action_counts.is_empty() {
        println!("── top actions ──");
        for (name, n) in s.action_counts.iter().take(8) {
            println!("  {name:20} {n}");
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Run {
            goal,
            steps,
            no_save,
        } => {
            let mut kernel = open_kernel(&cli)?;
            if *no_save {
                kernel.autosave = false;
            }
            let report = kernel.run(goal, *steps)?;
            if !*no_save {
                kernel.save()?;
            }
            println!("steps            : {}", report.steps);
            println!("patches_accepted : {}", report.patches_accepted);
            println!("early_avg_err    : {:.4}", report.early_avg_err);
            println!("final_avg_err    : {:.4}", report.final_avg_err);
            println!("final_damping    : {:.4}", report.final_damping);
            println!("error_decreased  : {}", report.error_decreased);
            if !report.last_actions.is_empty() {
                println!("last_actions     : {}", report.last_actions.join(", "));
            }
        }
        Commands::Status { json } => {
            let kernel = open_kernel(&cli)?;
            let st = kernel.status();
            if *json {
                println!("{}", serde_json::to_string_pretty(&st)?);
            } else {
                println!("schema_version     : {}", st.schema_version);
                println!("total_steps        : {}", st.total_steps);
                println!("avg_prediction_err : {:.4}", st.avg_prediction_error);
                println!("adaptation_score   : {:.4}", st.adaptation_score);
                println!("error_damping      : {:.4}", st.error_damping);
                println!("explore_bonus      : {:.4}", st.explore_bonus);
                println!("risk_tolerance     : {:.4}", st.risk_tolerance);
                println!("active_modules     : {}", st.active_modules);
                println!("strategy_count     : {}", st.strategy_count);
                println!("case_count         : {}", st.case_count);
                println!("checkpoint_count   : {}", st.checkpoint_count);
                println!("in_fixed_point     : {}", st.in_fixed_point);
                println!("dwell_steps        : {}", st.dwell_steps);
                println!("last_alignment     : {:.4}", st.last_alignment_score);
                println!("max_risk           : {:.2}", st.max_risk);
                println!("self_modify        : {}", st.self_modify_enabled);
                println!("l2_enabled        : {}", st.l2_enabled);
                println!("l2_cooldown       : {}", st.l2_commit_cooldown);
                println!("l2_consequence    : {:.4}", st.last_l2_consequence_score);
                println!("l2_defensive      : {}", st.l2_defensive_mode);
                println!("l2_negative_run   : {}", st.l2_negative_consequence_streak);
                if let Some(d) = &st.last_l2_decision {
                    println!(
                        "last_l2_decision  : arm={} bias={} mean={:.4} bonus={:.4} reason={:?}",
                        d.selected_arm,
                        d.selected_bias,
                        d.mean_reward,
                        d.exploration_bonus,
                        d.reason
                    );
                }
                println!("state_path         : {}", st.state_path);
            }
        }
        Commands::Reset { yes } => {
            if !*yes {
                anyhow::bail!("pass --yes to confirm reset (deletes state file)");
            }
            let mut kernel = open_kernel(&cli)?;
            kernel.reset()?;
            println!("reset ok; state cleared at {}", kernel.state_path.display());
        }
        Commands::Demo { steps } => {
            println!("Omega Kernel Demo (S3: audit · checkpoint · consequence)\n");
            // Keep the default demo isolated, including its sibling audit and
            // trajectory files. An explicit --state or config path must be
            // honored so a subsequent status/audit command reads the same run.
            let mut kernel = if cli.state.is_some() || cli.config.is_some() {
                open_kernel(&cli)?
            } else {
                let demo_dir = std::env::temp_dir()
                    .join(format!("omega-demo-{}", omega_cognitive_kernel::new_uuid()));
                std::fs::create_dir_all(&demo_dir)?;
                Kernel::with_state_path(demo_dir.join("state.json"))
            };
            let demo_path = kernel.state_path.clone();
            kernel.autosave = true;
            kernel.dump_trajectory = true;
            let report =
                kernel.run("maintain coherent self-model while acting in world", *steps)?;
            kernel.save()?;
            println!("steps            : {}", report.steps);
            println!("patches_accepted : {}", report.patches_accepted);
            println!("early_avg_err    : {:.4}", report.early_avg_err);
            println!("final_avg_err    : {:.4}", report.final_avg_err);
            println!("final_damping    : {:.4}", report.final_damping);
            println!("error_decreased  : {}", report.error_decreased);
            println!("\n── audit.jsonl (tail) ──");
            match kernel.audit_tail(12) {
                Ok(rows) if rows.is_empty() => println!("(empty)"),
                Ok(rows) => {
                    for r in rows {
                        println!("{}  {:?}", r.ts.to_rfc3339(), r.event);
                    }
                }
                Err(e) => println!("(audit error: {e})"),
            }
            println!("\nstate: {}", demo_path.display());
        }
        Commands::Audit { tail, summary } => {
            let kernel = open_kernel(&cli)?;
            let log = AuditLog::beside_state(&kernel.state_path);
            if *summary {
                let policy = load_intervention(&cli);
                print_audit_summary(&log, &policy)?;
            } else {
                let rows = kernel.audit_tail(*tail)?;
                if rows.is_empty() {
                    println!("(empty audit log)");
                }
                for r in rows {
                    println!("{}  {:?}", r.ts.to_rfc3339(), r.event);
                }
            }
        }
        Commands::Replay { path, tail } => {
            let traj = if let Some(p) = path {
                p.clone()
            } else {
                let kernel = open_kernel(&cli)?;
                let mut p = kernel.state_path.clone();
                p.set_file_name("trajectory.jsonl");
                p
            };
            let sum = replay_trajectory_file(&traj, *tail)?;
            println!("trajectory : {}", sum.path);
            println!("rows       : {}", sum.rows);
            println!("patches    : {}", sum.patches);
            if let Some(e) = sum.first_avg_err {
                println!("first_err  : {e:.4}");
            }
            if let Some(e) = sum.last_avg_err {
                println!("last_err   : {e:.4}");
            }
            if !sum.action_counts.is_empty() {
                println!("── actions ──");
                for (a, n) in &sum.action_counts {
                    println!("  {a:16} {n}");
                }
            }
            if !sum.tail.is_empty() {
                println!("── tail ──");
                for r in &sum.tail {
                    let err = r
                        .avg_err
                        .map(|e| format!("{e:.3}"))
                        .unwrap_or_else(|| "-".into());
                    println!(
                        "step={:<5} action={:<12} patched={} avg_err={} l2={}",
                        r.step,
                        r.action,
                        r.patched,
                        err,
                        r.l2_decision
                            .as_ref()
                            .map(|d| format!(
                                "arm={} mean={:.3} bonus={} reason={:?}",
                                d.selected_arm, d.mean_reward, d.exploration_bonus, d.reason
                            ))
                            .unwrap_or_else(|| "-".into())
                    );
                }
            }
        }
        Commands::Checkpoints => {
            let kernel = open_kernel(&cli)?;
            let list = kernel.list_checkpoints()?;
            if list.is_empty() {
                println!("(no checkpoints)");
            }
            for m in list {
                println!(
                    "{}  step={}  damp={:.2}  err={:.3}  label={}",
                    m.id, m.step, m.damping, m.avg_err, m.label
                );
            }
        }
        Commands::Checkpoint { label } => {
            let mut kernel = open_kernel(&cli)?;
            let meta = kernel.checkpoint(label)?;
            println!("checkpoint created: {} (step={})", meta.id, meta.step);
        }
        Commands::Rollback { id } => {
            let id = Uuid::parse_str(id).map_err(|e| anyhow::anyhow!("invalid uuid: {e}"))?;
            let mut kernel = open_kernel(&cli)?;
            kernel.rollback(id)?;
            kernel.save()?;
            println!("rolled back to {id}");
            println!("now at step {}", kernel.inner.total_steps);
        }
        Commands::Task { episodes, steps } => {
            let mut kernel = open_kernel(&cli)?;
            println!("bandit task  episodes={episodes} steps/ep={steps}");
            let report = run_bandit_task(&mut kernel, *episodes, *steps)?;
            kernel.save()?;
            println!("mean_reward       : {:.3}", report.mean_reward);
            println!("best_arm_rate     : {:.3}", report.best_arm_rate);
            println!("final_damping     : {:.3}", report.final_damping);
            println!("explore_bonus     : {:.3}", report.final_explore_bonus);
            println!("risk_tolerance    : {:.3}", report.final_risk_tolerance);
            println!("patches_accepted  : {}", report.patches_accepted);
            println!("error_decreased   : {}", report.error_decreased);
            println!("── strategy decay ──");
            println!("strategies        : {}", report.strategy_count);
            println!("idle_strategies   : {}", report.idle_strategy_count);
            println!("mean_weight       : {:.3}", report.mean_strategy_weight);
            println!("mean_idle_steps   : {:.2}", report.mean_idle_steps);
            println!("max_idle_steps    : {}", report.max_idle_steps);
            println!("mean_idle_weight  : {:.3}", report.mean_idle_weight);
            println!("mean_active_weight: {:.3}", report.mean_active_weight);
        }
        Commands::BenchmarkL2 {
            scenario,
            runs,
            episodes,
            steps,
            seed,
            commit_threshold,
            commit_cooldown,
            negative_reward_required,
            spectral_control,
            no_purity_prior,
            no_reverse_brake,
            no_history_discount,
            diagnostics,
            json,
        } => {
            let commit_threshold =
                commit_threshold.unwrap_or(if scenario == "degraded" { 0.50 } else { 0.35 });
            let commit_cooldown = commit_cooldown.unwrap_or(3);
            let negative_reward_required =
                negative_reward_required.unwrap_or(if scenario == "stable" { 5 } else { 3 });
            let report = run_l2_benchmark_scenario_with_gate(
                scenario,
                *runs,
                *episodes,
                *steps,
                *seed,
                commit_threshold,
                commit_cooldown,
                negative_reward_required,
                (*spectral_control || *diagnostics).then_some(L2ExperimentSettings {
                    spectral_control: *spectral_control,
                    purity_prior: !*no_purity_prior,
                    reverse_brake: !*no_reverse_brake,
                    history_discount: !*no_history_discount,
                }),
            )?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                let base = &report.baseline_l2_disabled;
                let l2 = &report.treatment_l2_enabled;
                println!("L2 benchmark (same seeds, baseline vs treatment)");
                println!("runs              : {}", report.runs);
                println!("scenario          : {}", report.scenario);
                println!(
                    "episodes/steps    : {}/{}",
                    report.episodes, report.steps_per_episode
                );
                println!("seed              : {}", report.seed);
                println!("                    baseline      L2 enabled     delta");
                println!(
                    "mean_reward       : {:>10.4}    {:>10.4}    {:+.4}",
                    base.mean_reward, l2.mean_reward, report.delta_mean_reward
                );
                println!(
                    "best_arm_rate     : {:>10.4}    {:>10.4}    {:+.4}",
                    base.best_arm_rate, l2.best_arm_rate, report.delta_best_arm_rate
                );
                println!(
                    "error_delta       : {:>10.4}    {:>10.4}",
                    base.mean_error_delta, l2.mean_error_delta
                );
                println!(
                    "error_decrease_rt  : {:>10.4}    {:>10.4}    {:+.4}",
                    base.error_decrease_rate,
                    l2.error_decrease_rate,
                    report.delta_error_decrease_rate
                );
                println!(
                    "patches/run       : {:>10.4}    {:>10.4}",
                    base.mean_patches_accepted, l2.mean_patches_accepted
                );
                println!(
                    "fp enters/run     : {:>10.4}    {:>10.4}",
                    base.mean_fixed_point_enters, l2.mean_fixed_point_enters
                );
                println!(
                    "fp exits/run      : {:>10.4}    {:>10.4}",
                    base.mean_fixed_point_exits, l2.mean_fixed_point_exits
                );
                println!(
                    "fp max dwell/run  : {:>10.4}    {:>10.4}",
                    base.mean_max_fixed_point_dwell, l2.mean_max_fixed_point_dwell
                );
                println!(
                    "proposals/run     : {:>10.4}    {:>10.4}",
                    base.mean_l2_proposals, l2.mean_l2_proposals
                );
                println!(
                    "commits/run       : {:>10.4}    {:>10.4}",
                    base.mean_l2_commits, l2.mean_l2_commits
                );
                println!(
                    "l2 nudges/run     : {:>10.4}    {:>10.4}",
                    base.mean_l2_nudges, l2.mean_l2_nudges
                );
                println!(
                    "action switches   : {:>10.4}    {:>10.4}",
                    base.mean_action_switches, l2.mean_action_switches
                );
                println!(
                    "post-L2 Δaction   : {:>10.4}    {:>10.4}",
                    base.mean_post_commit_action_changes, l2.mean_post_commit_action_changes
                );
                println!(
                    "post-L2 Δarm      : {:>10.4}    {:>10.4}",
                    base.mean_post_commit_arm_changes, l2.mean_post_commit_arm_changes
                );
                println!(
                    "distinct actions  : {:>10.4}    {:>10.4}",
                    base.mean_distinct_actions, l2.mean_distinct_actions
                );
                println!(
                    "consequence       : {:>10.4}    {:>10.4}",
                    base.mean_l2_consequence_score, l2.mean_l2_consequence_score
                );
                println!(
                    "final alignment   : {:>10.4}    {:>10.4}",
                    base.mean_final_alignment_score, l2.mean_final_alignment_score
                );
                println!(
                    "recovery steps    : {:>10?}    {:>10?}",
                    base.mean_recovery_steps, l2.mean_recovery_steps
                );
                println!(
                    "L1 rollbacks/run  : {:>10.4}    {:>10.4}",
                    base.mean_l1_rollback_count, l2.mean_l1_rollback_count
                );
                println!(
                    "Dayan residual    : {:>10.4}    {:>10.4}",
                    base.mean_final_dayan_residual, l2.mean_final_dayan_residual
                );
            }
        }
        Commands::BenchmarkL1l2 {
            scenario,
            runs,
            episodes,
            steps,
            seed,
            commit_threshold,
            commit_cooldown,
            negative_reward_required,
            json,
        } => {
            let report = run_l1_l2_factorial_benchmark(
                scenario,
                *runs,
                *episodes,
                *steps,
                *seed,
                commit_threshold.unwrap_or(if scenario == "degraded" { 0.50 } else { 0.35 }),
                commit_cooldown.unwrap_or(3),
                negative_reward_required.unwrap_or(if scenario == "stable" { 5 } else { 3 }),
            )?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("L1/L2 factorial benchmark (common seeded environments)");
                println!(
                    "scenario={} runs={} episodes={} steps={} seed={}",
                    report.scenario,
                    report.runs,
                    report.episodes,
                    report.steps_per_episode,
                    report.seed
                );
                println!(
                    "arm        reward    best-arm  adjusted  patches  L1-block  commits/applied"
                );
                for (arm, aggregate) in report.arm_order.iter().zip(&report.aggregates) {
                    println!(
                        "{:<7} {:>9.4}  {:>9.4}  {:>8.4}  {:>7.2}  {:>8.2}  {:.2}/{:.2}",
                        arm.label(),
                        aggregate.mean_reward,
                        aggregate.best_arm_rate,
                        aggregate.mean_adjusted_reward,
                        aggregate.mean_patches_accepted,
                        aggregate.mean_l1_blocked_by_l2_count,
                        aggregate.mean_l2_commits,
                        aggregate.mean_committed_policies_applied,
                    );
                }
                for (name, effect) in [
                    ("L1-L0", &report.l1_effect),
                    ("L2-L0", &report.l2_effect),
                    ("L1+L2-L0", &report.joint_effect),
                    ("interaction", &report.interaction_effect),
                ] {
                    println!(
                        "{name:<10} reward={:+.6} t95=[{:+.6}, {:+.6}] adjusted={:+.6}",
                        effect.reward.mean,
                        effect.reward.ci95_low.unwrap_or(f64::NAN),
                        effect.reward.ci95_high.unwrap_or(f64::NAN),
                        effect.adjusted_reward.mean,
                    );
                }
                println!("Reward deltas are descriptive; patches/commits are not benefits.");
            }
        }
        Commands::BenchmarkL2Gain {
            runs,
            episodes,
            steps,
            smoke,
            json,
        } => {
            let report = if *smoke {
                run_l2_gain_matrix(2, 8, 8)?
            } else {
                run_l2_gain_matrix(*runs, *episodes, *steps)?
            };
            if *json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("L2 gain matrix (paired L2-only minus L0 external reward)");
                println!(
                    "seeds={} discovery={} validation={} horizon={}x{}",
                    report.runs_per_scenario,
                    report.discovery_seed,
                    report.validation_seed,
                    report.episodes,
                    report.steps_per_episode,
                );
                println!(
                    "scale={} kernel runs / {} decision steps",
                    report.kernel_runs, report.decision_steps
                );
                for factorial in &report.reports {
                    let seed_set = if factorial.seed == L2_GAIN_DISCOVERY_SEED {
                        "discovery"
                    } else {
                        "validation "
                    };
                    println!(
                        "{} {:<14} reward={:+.6} t95=[{:+.6}, {:+.6}] applied={:.2}",
                        seed_set,
                        factorial.scenario,
                        factorial.l2_effect.reward.mean,
                        factorial.l2_effect.reward.ci95_low.unwrap_or(f64::NAN),
                        factorial.l2_effect.reward.ci95_high.unwrap_or(f64::NAN),
                        factorial.aggregates[2].mean_committed_policies_applied,
                    );
                }
                let assessment = &report.assessment;
                println!(
                    "primary discovery={} validation={} safety discovery={} validation={} executed={}",
                    assessment.discovery_primary_passed,
                    assessment.validation_reproduced,
                    assessment.discovery_safety_passed,
                    assessment.validation_safety_passed,
                    assessment.mechanism_precondition_met,
                );
                println!("reproducible_gain={}", assessment.reproducible_gain);
                println!("conclusion: {}", assessment.conclusion);
            }
        }
        Commands::StrategyWeight { name, weight } => {
            let mut kernel = open_kernel(&cli)?;
            if kernel.set_strategy_weight(name, *weight) {
                kernel.save()?;
                println!("ok  {name} weight -> {weight:.3}");
            } else {
                anyhow::bail!("unknown strategy: {name}");
            }
        }
        Commands::AuditPoll { offset } => {
            let kernel = open_kernel(&cli)?;
            let log = AuditLog::beside_state(&kernel.state_path);
            let (rows, next) = log.poll_after(*offset)?;
            println!("offset_in  : {offset}");
            println!("offset_out : {next}");
            println!("new_events : {}", rows.len());
            for r in &rows {
                println!("{}  {:?}", r.ts.to_rfc3339(), r.event);
            }
        }
        Commands::StepExternal {
            step_id,
            error,
            risk,
            outcome,
            shadow,
            json,
            no_save,
        } => {
            let mut cfg = resolve_effective_config(&cli)?;
            if *shadow {
                cfg.l1_shadow_mode = true;
            }
            let mut kernel = open_kernel(&cli)?;
            if *no_save {
                kernel.autosave = false;
            }
            let mut session = ClosedLoopSession::from_kernel(kernel, &cfg);
            let telemetry = StepTelemetry {
                step_id: *step_id,
                prediction_error: *error,
                observed_risk: *risk,
                signal_window: vec![*error],
                last_outcome: outcome.map(Into::into),
            };
            let directive = session
                .step_external(telemetry)
                .map_err(anyhow::Error::msg)?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&directive)?);
            } else {
                println!("step_id     : {}", directive.step_id);
                println!("l1_gate     : {:?}", directive.l1_gate);
                println!("keep_steady : {}", directive.keep_steady);
                println!("rule_patch  : {:?}", directive.rule_patch);
                println!("l2_prior    : {:?}", directive.l2_prior);
                println!("rollback_to : {:?}", directive.rollback_to_checkpoint);
            }
        }
        Commands::BenchmarkLoops {
            scenario,
            steps,
            json,
        } => {
            let report = run_closed_loop_matrix_benchmark((*scenario).into(), *steps);
            if *json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "scenario={:?} steps_per_preset={}",
                    report.scenario, report.steps_per_preset
                );
                println!(
                    "{:<15} {:<10} {:>6} {:>10} {:>8} {:>9} {:>7} {:>11}",
                    "preset", "mode", "steps", "mean_err", "patches", "rollbacks", "steady", "avg_step_us"
                );
                for row in &report.rows {
                    println!(
                        "{:<15} {:<10} {:>6} {:>10.4} {:>8} {:>9} {:>7} {:>11.2}",
                        row.preset,
                        row.mode,
                        row.steps,
                        row.mean_error,
                        row.patches_emitted,
                        row.rollbacks,
                        row.steady_steps,
                        row.avg_step_us
                    );
                }
            }
        }
    }
    Ok(())
}
