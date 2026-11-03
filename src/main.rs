use clap::{Parser, Subcommand};
use omega_cognitive_kernel::api::Kernel;
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
    Audit {
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

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match &cli.command {
        Commands::Run {
            goal,
            steps,
            no_save,
        } => {
            let mut kernel = Kernel::open(state_path(&cli).as_deref())?;
            kernel.autosave = !no_save;
            println!("goal   : {goal}");
            println!("steps  : {steps}");
            println!("state  : {}", kernel.state_path.display());
            println!();
            for i in 1..=*steps {
                let r = kernel.step(goal)?;
                let act = r
                    .trajectory
                    .actions
                    .first()
                    .map(|a| a.name.as_str())
                    .unwrap_or("-");
                println!(
                    "step {i:>2} │ act={act:<9} │ patched={:<5} │ damp={:.2} │ avg_err={:.3}",
                    r.l1.patched,
                    kernel.inner.l0.memory.cognitive_effect.error_damping,
                    kernel.inner.l0.memory.cas.avg_prediction_error,
                );
                if let Some(d) = r.l1.effect_damping {
                    println!("         └─ rule committed: error_damping → {d:.2}");
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
        }
        Commands::Status => {
            let kernel = Kernel::open(state_path(&cli).as_deref())?;
            let st = kernel.status();
            println!("state_path     : {}", st.state_path);
            println!("total_steps    : {}", st.total_steps);
            println!("avg_pred_error : {:.4}", st.avg_prediction_error);
            println!("adaptation     : {:.4}", st.adaptation_score);
            println!("error_damping  : {:.4}", st.error_damping);
            println!("active_modules : {}", st.active_modules);
            println!("strategies     : {}", st.strategy_count);
            println!("cases          : {}", st.case_count);
            println!("checkpoints    : {}", st.checkpoint_count);
        }
        Commands::Reset { yes } => {
            if !yes {
                eprintln!("Refusing to reset without --yes");
                std::process::exit(2);
            }
            let mut kernel = Kernel::open(state_path(&cli).as_deref())?;
            let path = kernel.state_path.display().to_string();
            kernel.reset()?;
            println!("reset ok — removed state at {path}");
        }
        Commands::Demo { steps } => {
            println!("Omega Kernel Demo (S3: audit · checkpoint · consequence)\n");
            let demo_path = std::env::temp_dir().join("omega-demo-state.json");
            let _ = std::fs::remove_file(&demo_path);
            let mut kernel = Kernel::with_state_path(&demo_path);
            kernel.autosave = true;
            let goal = "maintain coherent self-model while acting in world";
            let report = kernel.run(goal, *steps)?;
            println!("steps            : {}", report.steps);
            println!("patches accepted : {}", report.patches_accepted);
            println!("early avg_err    : {:.3}", report.early_avg_err);
            println!("final avg_err    : {:.3}", report.final_avg_err);
            println!("final damping    : {:.3}", report.final_damping);
            println!(
                "error decreased  : {}",
                if report.error_decreased { "yes" } else { "no" }
            );
            println!("checkpoints      : {}", kernel.status().checkpoint_count);
            println!("state: {}", demo_path.display());
        }
        Commands::Audit { tail } => {
            let kernel = Kernel::open(state_path(&cli).as_deref())?;
            let rows = kernel.audit_tail(*tail)?;
            if rows.is_empty() {
                println!("(empty audit log)");
            }
            for r in rows {
                println!("{}  {:?}", r.ts.to_rfc3339(), r.event);
            }
        }
        Commands::Checkpoints => {
            let kernel = Kernel::open(state_path(&cli).as_deref())?;
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
            let kernel = Kernel::open(state_path(&cli).as_deref())?;
            let meta = kernel.checkpoint(label)?;
            println!("checkpoint created: {} (step={})", meta.id, meta.step);
        }
        Commands::Rollback { id } => {
            let id = Uuid::parse_str(id).map_err(|e| anyhow::anyhow!("invalid uuid: {e}"))?;
            let mut kernel = Kernel::open(state_path(&cli).as_deref())?;
            kernel.rollback(id)?;
            println!("rolled back to {id}");
            println!("now at step {}", kernel.inner.total_steps);
        }
        Commands::Task { episodes, steps } => {
            let mut kernel = Kernel::open(state_path(&cli).as_deref())?;
            println!("bandit task  episodes={episodes} steps/ep={steps}");
            let report = run_bandit_task(&mut kernel, *episodes, *steps)?;
            kernel.save()?;
            println!("mean_reward    : {:.3}", report.mean_reward);
            println!("best_arm_rate  : {:.3}", report.best_arm_rate);
            println!("final_damping  : {:.3}", report.final_damping);
            println!("error_decreased: {}", report.error_decreased);
        }
    }
    Ok(())
}
