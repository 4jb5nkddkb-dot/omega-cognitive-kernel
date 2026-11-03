use omega_cognitive_kernel::l0::{
    decision::ConstrainedDecisionMaker,
    executor::SandboxedExecutor,
    observer::MultiModalObserver,
    predictor::ModelBasedPredictor,
    L0WorldLayer,
};
use omega_cognitive_kernel::l1::L1CognitiveLayer;
use omega_cognitive_kernel::l2::L2MetaLayer;
use omega_cognitive_kernel::memory::MemoryFacade;
use omega_cognitive_kernel::orchestrator::OmegaOrchestrator;
use omega_cognitive_kernel::safety;
use std::collections::HashMap;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         Omega Cognitive Kernel v0.1.0 — S1             ║");
    println!("║   Consequence loop: memory + rules change behavior     ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let observer = MultiModalObserver::new();
    let predictor = ModelBasedPredictor;
    let decision = ConstrainedDecisionMaker;
    let executor = SandboxedExecutor::new(vec![
        "noop".into(),
        "observe".into(),
        "explore".into(),
        "exploit".into(),
        "stabilize".into(),
    ]);
    let memory = MemoryFacade::new();

    let l0 = L0WorldLayer {
        observer,
        predictor,
        decision,
        executor,
        memory,
    };

    let l1 = L1CognitiveLayer::new();
    let l2 = L2MetaLayer::new();

    let mut kernel = OmegaOrchestrator {
        l0,
        l1,
        l2,
        biases: HashMap::new(),
        constraints: safety::default_constraints(),
        total_steps: 0,
    };

    let goal = "maintain coherent self-model while acting in world";
    let mut first_err: Option<f64> = None;
    let mut last_err = 0.0;

    for step in 1..=12 {
        match kernel.run_once(goal) {
            Ok(result) => {
                let err = kernel.l0.memory.cas.avg_prediction_error;
                if first_err.is_none() {
                    first_err = Some(err);
                }
                last_err = err;
                let action = result
                    .trajectory
                    .actions
                    .first()
                    .map(|a| a.name.as_str())
                    .unwrap_or("-");
                println!(
                    "step {:>2} │ act={:<9} │ patched={:<5} │ damp={:.2} │ avg_err={:.3} │ align={:.2}",
                    step,
                    action,
                    result.l1.patched,
                    kernel.l0.memory.cognitive_effect.error_damping,
                    err,
                    result.l2.alignment_score,
                );
                if let Some(d) = result.l1.effect_damping {
                    println!("         └─ rule committed: error_damping → {:.2}", d);
                }
                if let Some(reason) = &result.l1.reason {
                    println!("         └─ patch rejected: {}", reason);
                }
                if result.l2.entered_fixed_point {
                    println!("         └─ ★ entered omega=dayan fixed point mode");
                }
            }
            Err(e) => {
                eprintln!("step {:>2} │ error: {}", step, e);
            }
        }
    }

    println!("\n── S1 consequence summary ──");
    if let Some(fe) = first_err {
        println!("early avg_err  : {:.3}", fe);
        println!("final avg_err  : {:.3}", last_err);
        println!(
            "active damping : {:.3}",
            kernel.l0.memory.cognitive_effect.error_damping
        );
        if last_err < fe {
            println!("result         : error decreased (self-modification had consequence)");
        } else {
            println!("result         : error did not decrease (inspect rules / thresholds)");
        }
    }

    println!("\n── Active cognitive modules ──");
    for (k, v) in &kernel.l1.store.active {
        println!("• {} ({} bytes)", k, v.len());
    }

    println!("\n── Final biases ──");
    for (k, v) in &kernel.biases {
        println!("• {}: {:.3}", k, v);
    }

    println!("\nDemo finished.");
    Ok(())
}
