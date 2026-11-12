use clap::{Parser, Subcommand};
use omega_cognitive_kernel::api::Kernel;
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
        if let Some(ref p) = cli.state {
            kernel.state_path = p.clone();
        } else if cfg.state_path.is_some() && kernel.state_path.exists() {
            let _ = kernel.load();
        }
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
    if !s.action_counts.is_empty() {
        println!("── actions ──");
        for (name, n) in &s.action_counts {
            println!("  {name:<12} {n}");
        }
    }
    if let Some(a) = &s.last_action {
        println!("── last step ──");
        println!("action     : {a}");
        if let Some(e) = s.last_avg_err {
            println!("avg_err    : {e:.4}");
        }
        if let Some(d) = s.last_damping {
            println!("damping    : {d:.4}");
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match &cli.command {
        Commands::Run {
            goal,
            steps,
            no_save,
        } => {
            let mut kernel = open_kernel(&cli)?;
            kernel.autosave = !no_save;
            let goal = if goal == "maintain coherent self-model while acting in world" {
                if let Some(ref cfg_path) = cli.config {
                    if let Ok(cfg) = KernelConfig::load(cfg_path) {
                        cfg.default_goal.unwrap_or_else(|| goal.clone())
                    } else {
                        goal.clone()
                    }
                } else {
                    goal.clone()
                }
            } else {
                goal.clone()
            };
            println!("goal   : {goal}");
            println!("steps  : {steps}");
            println!("state  : {}", kernel.state_path.display());
            println!(
                "self_modify : {}  dump_trajectory : {}",
                kernel.inner.l1.self_modify_enabled, kernel.dump_trajectory
            );
            println!();
            for i in 1..=*steps {
                let r = kernel.step(&goal)?;
                let act = r
                    .trajectory
                    .actions
                    .first()
                    .map(|a| a.name.as_str())
                    .unwrap_or("-");
                let effect = &kernel.inner.l0.memory.cognitive_effect;
                let fp = if r.l2.in_fixed_point { "FP" } else { "  " };
                println!(
                    "step {i:>2} │ act={act:<9} │ patched={:<5} │ damp={:.2} │ avg_err={:.3} │ {fp} align={:.2}",
                    r.l1.patched,
                    effect.error_damping,
                    kernel.inner.l0.memory.cas.avg_prediction_error,
                    r.l2.alignment_score,
                );
                if let Some(d) = r.l1.effect_damping {
                    println!(
                        "         └─ rule committed: damp→{d:.2}  explore_bonus={:.2}  risk_tol={:.2}",
                        effect.explore_bonus, effect.risk_tolerance
                    );
                }
                if r.l2.entered_fixed_point {
                    println!(
                        "         └─ entered fixed-point (align={:.2})",
                        r.l2.alignment_score
                    );
                }
                if r.l2.exited_fixed_point {
                    println!("         └─ exited fixed-point");
                }
                if r.l2.effect_nudge.is_some() {
                    println!(
                        "         └─ L2 soft-nudge: explore_bonus={:.2} risk_tol={:.2}",
                        effect.explore_bonus, effect.risk_tolerance
                    );
                }
            }
            if !no_save {
                kernel.save()?;
            }
            let st = kernel.status();
            println!("\n── status ──");
            println!(
                "steps={}  avg_err={:.3}  damp={:.3}  modules={}  ckpts={}",
                st.total_steps,
                st.avg_prediction_error,
                st.error_damping,
                st.active_modules,
                st.checkpoint_count
            );
            println!(
                "L2: in_fp={} dwell={} align={:.3}  max_risk={:.2}",
                st.in_fixed_point, st.dwell_steps, st.last_alignment_score, st.max_risk
            );
            println!(
                "self_modify={}  dump_trajectory={}",
                st.self_modify_enabled, kernel.dump_trajectory
            );
        }
        Commands::Status => {
            let kernel = open_kernel(&cli)?;
            let st = kernel.status();
            println!("state_path          : {}", st.state_path);
            println!("total_steps         : {}", st.total_steps);
            println!("avg_pred_error      : {:.4}", st.avg_prediction_error);
            println!("adaptation          : {:.4}", st.adaptation_score);
            println!("error_damping       : {:.4}", st.error_damping);
            println!("explore_bonus       : {:.4}", st.explore_bonus);
            println!("risk_tolerance      : {:.4}", st.risk_tolerance);
            println!("max_risk            : {:.4}", st.max_risk);
            println!("active_modules      : {}", st.active_modules);
            println!("strategies          : {}", st.strategy_count);
            println!("cases               : {}", st.case_count);
            println!("checkpoints         : {}", st.checkpoint_count);
            println!("self_modify_enabled : {}", st.self_modify_enabled);
            println!("dump_trajectory     : {}", kernel.dump_trajectory);
            println!("── L2 ──");
            println!("in_fixed_point      : {}", st.in_fixed_point);
            println!("dwell_steps         : {}", st.dwell_steps);
            println!("alignment_score     : {:.4}", st.last_alignment_score);
        }
        Commands::Reset { yes } => {
            if !yes {
                eprintln!("Refusing to reset without --yes");
                std::process::exit(2);
            }
            let mut kernel = open_kernel(&cli)?;
            let path = kernel.state_path.display().to_string();
            kernel.reset()?;
            println!("reset ok — removed state at {path}");
        }
        Commands::Demo { steps } => {
            println!("Omega Kernel Demo (S3: audit · checkpoint · consequence)\n");
            let demo_path = std::env::temp_dir().join("omega-demo-state.json");
            let _ = std::fs::remove_file(&demo_path);
            let mut kernel = Kernel::with_state_path(&demo_path);
            if let Some(ref cfg_path) = cli.config {
                let cfg = KernelConfig::load(cfg_path)?;
                kernel.apply_config(&cfg);
            }
            kernel.autosave = true;
            let goal = "maintain coherent self-model while acting in world";
            let report = kernel.run(goal, *steps)?;
            let effect = &kernel.inner.l0.memory.cognitive_effect;
            let st = kernel.status();
            println!("steps            : {}", report.steps);
            println!("patches accepted : {}", report.patches_accepted);
            println!("early avg_err    : {:.3}", report.early_avg_err);
            println!("final avg_err    : {:.3}", report.final_avg_err);
            println!("final damping    : {:.3}", report.final_damping);
            println!("explore_bonus    : {:.3}", effect.explore_bonus);
            println!("risk_tolerance   : {:.3}", effect.risk_tolerance);
            println!(
                "error decreased  : {}",
                if report.error_decreased { "yes" } else { "no" }
            );
            println!("checkpoints      : {}", st.checkpoint_count);
            println!(
                "L2 in_fp={} dwell={} align={:.3}",
                st.in_fixed_point, st.dwell_steps, st.last_alignment_score
            );
            println!(
                "self_modify={}  dump_trajectory={}",
                st.self_modify_enabled, kernel.dump_trajectory
            );
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
