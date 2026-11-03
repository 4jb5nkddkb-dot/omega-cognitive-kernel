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
    println!("║         Omega Cognitive Kernel v0.1.0                  ║");
    println!("║   L0 World · L1 Cognitive · L2 Meta (omega=dayan)      ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let observer = MultiModalObserver::new();
    let predictor = ModelBasedPredictor;
    let decision = ConstrainedDecisionMaker;
    let executor = SandboxedExecutor::new(vec!["noop".into(), "observe".into()]);
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

    for step in 1..=10 {
        match kernel.run_once(goal) {
            Ok(result) => {
                println!(
                    "step {:>2} │ patched={:<5} │ fixed_point={:<5} │ align={:.2} │ pred_power={:.3} │ avg_err={:.3}",
                    step,
                    result.l1.patched,
                    result.l2.in_fixed_point,
                    result.l2.alignment_score,
                    result.l2.metrics.predictive_power,
                    kernel.l0.memory.cas.avg_prediction_error
                );
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
