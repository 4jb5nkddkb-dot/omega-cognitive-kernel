//! Runtime-level smoke test for the wasm32-wasi library surface.
//! Build with:
//! cargo build --example wasm_runtime_smoke --no-default-features --target wasm32-wasip1

use omega_cognitive_kernel::Kernel;

fn main() {
    let mut source = Kernel::with_state_path("/tmp/omega-wasm-runtime-source.json");
    source.autosave = false;
    source.dump_trajectory = false;
    source.auto_checkpoint_on_patch = false;

    for _ in 0..4 {
        source
            .step("maintain coherent self-model omega dayan")
            .expect("source step should execute under WASI");
    }
    let source_steps = source.status().total_steps;
    let snapshot = source
        .snapshot_json()
        .expect("snapshot serialization should work under WASI");

    let mut restored = Kernel::with_state_path("/tmp/omega-wasm-runtime-restored.json");
    restored.autosave = false;
    restored.dump_trajectory = false;
    restored.auto_checkpoint_on_patch = false;
    restored
        .apply_snapshot_json(&snapshot)
        .expect("snapshot restore should work under WASI");
    assert_eq!(restored.status().total_steps, source_steps);

    restored
        .step("continue after wasm hot restore")
        .expect("restored kernel should continue under WASI");
    assert_eq!(restored.status().total_steps, source_steps + 1);

    println!(
        "wasm-runtime-smoke: ok steps_before={} steps_after={} snapshot_bytes={}",
        source_steps,
        restored.status().total_steps,
        snapshot.len()
    );
}
