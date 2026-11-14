use clap::{Parser, Subcommand};
use omega_cognitive_kernel::api::{replay_trajectory_file, Kernel};
use omega_cognitive_kernel::audit::AuditLog;
use omega_cognitive_kernel::config::{InterventionPolicy, KernelConfig};
use omega_cognitive_kernel::task::run_bandit_task;
use std::path::PathBuf;
use uuid::Uuid;

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
    Status,
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
}

fn state_path(cli: &Cli) -> Option<String> {
    cli.state.as_ref().map(|p| p.display().to_string())
}

fn load_intervention(cli: &Cli) -> InterventionPolicy {
    if let Some(ref cfg_path) = cli.config {
        if let Ok(cfg) = KernelConfig::load(cfg_path) {
            return cfg.intervention;
        }
    }
    InterventionPolicy::default()
}

fn open_kernel(cli: &Cli) -> anyhow::Result<Kernel> {
    let mut kernel = Kernel::open(state_path(cli).as_deref())?;
    if let Some(ref cfg_path) = cli.config {
        let cfg = KernelConfig::load(cfg_path)?;
        kernel.apply_config(&cfg);
        if let Some(ref s) = cli.state {
            kernel.state_path = s.clone();
        }
    }
    Ok(kernel)
}

fn print_intervention(p: &InterventionPolicy) {
    let mode = match p.mode {
        omega_cognitive_kernel::config::InterventionMode::Exploration => "exploration",
        omega_cognitive_kernel::config::InterventionMode::Strict => "strict",
    };
    let max = match p.max_level {
        omega_cognitive_kernel::config::InterventionLevel::None => "none",
        omega_cognitive_kernel::config::InterventionLevel::Log => "log",
        omega_cognitive_kernel::config::InterventionLevel::SuggestAlternative => "suggest",
        omega_cognitive_kernel::config::InterventionLevel::Block => "block",
    };
    let eff = match p.effective_max_level() {
        omega_cognitive_kernel::config::InterventionLevel::None => "none",
        omega_cognitive_kernel::config::InterventionLevel::Log => "log",
        omega_cognitive_kernel::config::InterventionLevel::SuggestAlternative => "suggest",
        omega_cognitive_kernel::config::InterventionLevel::Block => "block",
    };
    println!("mode              : {mode}");
    println!("max_level         : {max}");
    println!("effective_max     : {eff}");
    println!("friction_threshold: {}", p.friction_threshold);
    println!("audit_escalations : {}", p.audit_escalations);
}

fn print_audit_summary(log: &AuditLog, policy: &InterventionPolicy) -> anyhow::Result<()> {
    let sum = log.summarize()?;
    println!("╔══════════════════════════════════════════════╗");
    println!("║           Omega audit panel                  ║");
    println!("╚══════════════════════════════════════════════╝");
    println!("path             : {}", log.path.display());
    println!("total_events     : {}", sum.total_events);
    println!("steps            : {}", sum.steps);
    println!("patches_accepted : {}", sum.patches_accepted);
    println!("patches_rejected : {}", sum.patches_rejected);
    println!("checkpoints      : {}", sum.checkpoints);
    println!("rollbacks        : {}", sum.rollbacks);
    println!("task_episodes    : {}", sum.task_episodes);
    println!("tools_allowed    : {}", sum.tools_allowed);
    println!("tools_denied     : {}", sum.tools_denied);
    if let Some(ts) = sum.first_ts {
        println!("first_ts         : {}", ts.to_rfc3339());
    }
    if let Some(ts) = sum.last_ts {
        println!("last_ts          : {}", ts.to_rfc3339());
    }
    if let Some(a) = &sum.last_action {
        println!("last_action      : {a}");
    }
    if let Some(e) = sum.last_avg_err {
        println!("last_avg_err     : {e:.4}");
    }
    if let Some(d) = sum.last_damping {
        println!("last_damping     : {d:.4}");
    }
    if !sum.action_counts.is_empty() {
        println!("── action counts ──");
        for (a, c) in sum.action_counts.iter().take(12) {
            println!("  {a:20} {c}");
        }
    }
    println!("── intervention ──");
    print_intervention(policy);
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
                println!(
                    "last_actions     : {}",
                    report.last_actions.join(", ")
                );
            }
            let st = kernel.status();
            println!("schema_version   : {}", st.schema_version);
            println!("self_modify      : {}", st.self_modify_enabled);
            println!("state            : {}", st.state_path);
        }
        Commands::Status => {
            let kernel = open_kernel(&cli)?;
            let st = kernel.status();
            println!("schema_version      : {}", st.schema_version);
            println!("total_steps         : {}", st.total_steps);
            println!("avg_prediction_error: {:.4}", st.avg_prediction_error);
            println!("adaptation_score    : {:.4}", st.adaptation_score);
            println!("error_damping       : {:.4}", st.error_damping);
            println!("explore_bonus       : {:.4}", st.explore_bonus);
            println!("risk_tolerance      : {:.4}", st.risk_tolerance);
            println!("active_modules      : {}", st.active_modules);
            println!("strategy_count      : {}", st.strategy_count);
            println!("case_count          : {}", st.case_count);
            println!("checkpoint_count    : {}", st.checkpoint_count);
            println!("in_fixed_point      : {}", st.in_fixed_point);
            println!("dwell_steps         : {}", st.dwell_steps);
            println!("last_alignment      : {:.4}", st.last_alignment_score);
            println!("max_risk            : {:.2}", st.max_risk);
            println!("self_modify_enabled : {}", st.self_modify_enabled);
            println!("state_path          : {}", st.state_path);
        }
        Commands::Reset { yes } => {
            if !yes {
                anyhow::bail!("pass --yes to confirm reset");
            }
            let mut kernel = open_kernel(&cli)?;
            kernel.reset()?;
            println!("reset ok — state cleared at {}", kernel.state_path.display());
        }
        Commands::Demo { steps } => {
            let demo_path = std::env::temp_dir().join("omega-demo-state.json");
            let mut kernel = Kernel::with_state_path(&demo_path);
            kernel.autosave = true;
            kernel.dump_trajectory = true;
            println!("Omega Kernel Demo (S3: audit · checkpoint · consequence)\n");
            let report = kernel.run(
                "maintain coherent self-model while acting in world",
                *steps,
            )?;
            kernel.save()?;
            println!("steps            : {}", report.steps);
            println!("patches_accepted : {}", report.patches_accepted);
            println!("early_avg_err    : {:.4}", report.early_avg_err);
            println!("final_avg_err    : {:.4}", report.final_avg_err);
            println!("final_damping    : {:.4}", report.final_damping);
            println!("error_decreased  : {}", report.error_decreased);
            println!("state: {}", demo_path.display());
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
                for (a, c) in sum.action_counts.iter().take(10) {
                    println!("  {a:16} {c}");
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
                        "  step={:<6} {:12} patched={} err={}",
                        r.step, r.action, r.patched, err
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
            let kernel = open_kernel(&cli)?;
            let meta = kernel.checkpoint(label)?;
            println!("checkpoint created: {} (step={})", meta.id, meta.step);
        }
        Commands::Rollback { id } => {
            let id = Uuid::parse_str(id).map_err(|e| anyhow::anyhow!("invalid uuid: {e}"))?;
            let mut kernel = open_kernel(&cli)?;
            kernel.rollback(id)?;
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
    }
    Ok(())
}
