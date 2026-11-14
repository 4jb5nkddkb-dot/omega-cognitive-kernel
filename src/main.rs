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
    let mut kernel = if let Some(ref cfg_path) = cli.config {
        let mut k = Kernel::from_config_path(cfg_path)?;
        if let Some(ref s) = cli.state {
            k.state_path = s.clone();
            if s.exists() {
                k.load()?;
            }
        }
        k
    } else {
        Kernel::open(state_path(cli).as_deref())?
    };
    Ok(kernel)
}

fn print_intervention(p: &InterventionPolicy) {
    let mode = match p.mode.as_str() {
        "strict" => "strict",
        _ => "exploration",
    };
    let max = match p.max_level.as_str() {
        "block" => "block",
        "suggest_alternative" => "suggest_alternative",
        "log" => "log",
        _ => "none",
    };
    println!("mode               : {mode}");
    println!("max_level          : {max}");
    println!("friction_threshold : {}", p.friction_threshold);
    println!("audit_escalations  : {}", p.audit_escalations);
}

fn print_audit_summary(log: &AuditLog, policy: &InterventionPolicy) -> anyhow::Result<()> {
    let sum = log.summarize()?;
    println!("╔══════════════════════════════════════════════╗");
    println!("║           Omega audit panel                  ║");
    println!("╚══════════════════════════════════════════════╝");
    println!("events            : {}", sum.total_events);
    println!("steps             : {}", sum.steps);
    println!("patches accepted  : {}", sum.patches_accepted);
    println!("patches rejected  : {}", sum.patches_rejected);
    println!("checkpoints       : {}", sum.checkpoints);
    println!("rollbacks         : {}", sum.rollbacks);
    println!("task episodes     : {}", sum.task_episodes);
    println!("tools allowed     : {}", sum.tools_allowed);
    println!("tools denied      : {}", sum.tools_denied);
    if let Some(a) = &sum.last_action {
        println!("last action       : {a}");
    }
    if let Some(e) = sum.last_avg_err {
        println!("last avg_err      : {e:.4}");
    }
    if let Some(d) = sum.last_damping {
        println!("last damping      : {d:.4}");
    }
    println!("── top actions ──");
    for (name, n) in sum.action_counts.iter().take(8) {
        println!("  {name:20} {n}");
    }
    println!("── intervention ──");
    print_intervention(policy);
    Ok(())
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

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
            println!("steps             : {}", report.steps);
            println!("patches accepted  : {}", report.patches_accepted);
            println!("early avg err     : {:.4}", report.early_avg_err);
            println!("final avg err     : {:.4}", report.final_avg_err);
            println!("final damping     : {:.4}", report.final_damping);
            println!("error decreased   : {}", report.error_decreased);
            if !report.last_actions.is_empty() {
                println!("last actions      : {}", report.last_actions.join(", "));
            }
        }
        Commands::Status => {
            let kernel = open_kernel(&cli)?;
            let s = kernel.status();
            println!("schema_version    : {}", s.schema_version);
            println!("total_steps       : {}", s.total_steps);
            println!("avg_pred_error    : {:.4}", s.avg_prediction_error);
            println!("adaptation_score  : {:.4}", s.adaptation_score);
            println!("error_damping     : {:.4}", s.error_damping);
            println!("explore_bonus     : {:.4}", s.explore_bonus);
            println!("risk_tolerance    : {:.4}", s.risk_tolerance);
            println!("active_modules    : {}", s.active_modules);
            println!("strategy_count    : {}", s.strategy_count);
            println!("case_count        : {}", s.case_count);
            println!("checkpoint_count  : {}", s.checkpoint_count);
            println!("in_fixed_point    : {}", s.in_fixed_point);
            println!("dwell_steps       : {}", s.dwell_steps);
            println!("last_alignment    : {:.4}", s.last_alignment_score);
            println!("max_risk          : {:.2}", s.max_risk);
            println!("self_modify       : {}", s.self_modify_enabled);
            println!("state_path        : {}", s.state_path);
        }
        Commands::Reset { yes } => {
            if !*yes {
                anyhow::bail!("pass --yes to confirm reset (deletes state file)");
            }
            let mut kernel = open_kernel(&cli)?;
            kernel.reset()?;
            println!("reset complete — state cleared at {}", kernel.state_path.display());
        }
        Commands::Demo { steps } => {
            println!("Omega Kernel Demo (S3: audit · checkpoint · consequence)\n");
            let demo_path = std::env::temp_dir().join(format!("omega-demo-{}.json", Uuid::new_v4()));
            let mut kernel = Kernel::with_state_path(&demo_path);
            kernel.autosave = true;
            let report = kernel.run(
                "maintain coherent self-model while acting in world",
                *steps,
            )?;
            println!("steps             : {}", report.steps);
            println!("patches accepted  : {}", report.patches_accepted);
            println!("early avg err     : {:.4}", report.early_avg_err);
            println!("final avg err     : {:.4}", report.final_avg_err);
            println!("final damping     : {:.4}", report.final_damping);
            println!("error decreased   : {}", report.error_decreased);
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
                for (a, c) in &sum.action_counts {
                    println!("  {a:16} {c}");
                }
            }
            if !sum.tail.is_empty() {
                println!("── tail ({}) ──", sum.tail.len());
                for r in &sum.tail {
                    let err = r
                        .avg_err
                        .map(|e| format!("{e:.3}"))
                        .unwrap_or_else(|| "-".into());
                    println!(
                        "  #{:<5} {:12} patched={} err={}",
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
