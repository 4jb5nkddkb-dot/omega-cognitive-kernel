use clap::{Parser, Subcommand};
use omega_cognitive_kernel::api::Kernel;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "omega-kernel",
    version,
    about = "Omega Cognitive Kernel — self-improving cognitive runtime (L0/L1/L2)"
)]
struct Cli {
    /// Path to persisted state (default: .omega/state.json)
    #[arg(long, global = true)]
    state: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run N cognitive cycles (product demo / training loop)
    Run {
        /// High-level goal biasing the loop
        #[arg(long, default_value = "maintain coherent self-model while acting in world")]
        goal: String,
        /// Number of steps
        #[arg(long, default_value_t = 12)]
        steps: u32,
        /// Disable writing state after each step
        #[arg(long)]
        no_save: bool,
    },
    /// Print current kernel status (loads state if present)
    Status,
    /// Delete persisted state and start clean
    Reset {
        /// Require explicit confirmation
        #[arg(long)]
        yes: bool,
    },
    /// One-shot narrative demo (run + summary suitable for sharing)
    Demo {
        #[arg(long, default_value_t = 12)]
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
                "steps={}  avg_err={:.3}  damp={:.3}  modules={}  strategies={}",
                st.total_steps,
                st.avg_prediction_error,
                st.error_damping,
                st.active_modules,
                st.strategy_count
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
            println!("╔══════════════════════════════════════════════════════════╗");
            println!("║     Omega Kernel — Product Demo (S2)                     ║");
            println!("║     Self-modification with measurable consequence        ║");
            println!("╚══════════════════════════════════════════════════════════╝\n");

            let demo_path = std::env::temp_dir().join("omega-demo-state.json");
            let _ = std::fs::remove_file(&demo_path);
            let mut kernel = Kernel::with_state_path(&demo_path);
            kernel.autosave = true;

            let goal = "maintain coherent self-model while acting in world";
            println!("Narrative:");
            println!("  1. Kernel starts with high uncertainty.");
            println!("  2. Errors trigger L1 cognitive patches (RuleEffect).");
            println!("  3. Damping + action preferences change L0 behavior.");
            println!("  4. Average prediction error should fall.\n");

            let report = kernel.run(goal, *steps)?;
            println!("── results ──");
            println!("steps            : {}", report.steps);
            println!("patches accepted : {}", report.patches_accepted);
            println!("early avg_err    : {:.3}", report.early_avg_err);
            println!("final avg_err    : {:.3}", report.final_avg_err);
            println!("final damping    : {:.3}", report.final_damping);
            println!(
                "error decreased  : {}",
                if report.error_decreased {
                    "yes ✓"
                } else {
                    "no"
                }
            );
            if let Some(last) = report.last_actions.last() {
                println!("last action      : {last}");
            }
            println!("\nState written to {}", demo_path.display());
            println!(
                "Re-run with: cargo run -- status --state {}",
                demo_path.display()
            );
        }
    }

    Ok(())
}
