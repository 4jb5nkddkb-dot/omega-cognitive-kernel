//! Async Observer Demo
//!
//! This example demonstrates how to use async observers with the kernel.
//! Requires the `async` feature: `cargo run --example async_observer_demo --features async`
//!
//! The async observers allow non-blocking I/O operations (file system, HTTP)
//! using tokio's async runtime.

#[cfg(feature = "async")]
use omega_cognitive_kernel::{AsyncDirSnapshotObserver, AsyncFileObserver, AsyncObserver};

#[cfg(feature = "async")]
#[tokio::main]
async fn main() -> omega_cognitive_kernel::Result<()> {
    // Initialize tracing for better observability
    tracing_subscriber::fmt::init();

    println!("=== Omega Cognitive Kernel - Async Observer Demo ===\n");

    // Create a temporary directory for async file observer demo
    let temp_dir = std::env::temp_dir().join(format!(
        "omega-async-demo-{}",
        omega_cognitive_kernel::new_uuid()
    ));
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| omega_cognitive_kernel::OmegaError::Other(anyhow::anyhow!(e)))?;

    // Write a sample file to observe
    let sample_file = temp_dir.join("sensor_data.json");
    std::fs::write(&sample_file, r#"{"temperature": 22.5, "humidity": 60}"#)
        .map_err(|e| omega_cognitive_kernel::OmegaError::Other(anyhow::anyhow!(e)))?;

    println!("1. AsyncFileObserver - Reading file asynchronously");
    println!("   Watching: {}", sample_file.display());

    // Demonstrate direct async observation
    let mut observer = AsyncFileObserver::new(&sample_file);
    match observer.observe_async().await {
        Ok(observations) => {
            println!("   ✓ Observed {} items:", observations.len());
            for obs in &observations {
                println!(
                    "     - Source: {}, Confidence: {:.2}",
                    obs.source, obs.confidence
                );
                println!("       Content: {}", obs.content);
            }
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }

    println!("\n2. AsyncDirSnapshotObserver - Directory snapshot asynchronously");
    println!("   Watching directory: {}", temp_dir.display());

    let mut dir_observer = AsyncDirSnapshotObserver::new(&temp_dir);
    match dir_observer.observe_async().await {
        Ok(observations) => {
            println!("   ✓ Observed {} items:", observations.len());
            for obs in &observations {
                println!("     - Source: {}", obs.source);
                println!("       Entry count: {}", obs.content["entry_count"]);
            }
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }

    println!("\n3. AsyncHttpObserver - HTTP request asynchronously (requires 'reqwest' feature)");
    println!("   For demo purposes, this is commented out to avoid network dependency:");
    println!("   ```rust");
    println!(
        "   let mut http_observer = AsyncHttpObserver::new(\"https://api.example.com/status\")"
    );
    println!("       .with_timeout(Duration::from_secs(5));");
    println!("   match http_observer.observe_async().await {{");
    println!("       Ok(observations) => {{ ... }}");
    println!("       Err(e) => println!(\"Error: {{}}\", e),");
    println!("   }}");
    println!("   ```");

    println!("\n4. AsyncObserverAdapter - Bridging sync observers to async");
    println!("   Any sync Observer can be wrapped for async use:");
    println!("   ```rust");
    println!("   use omega_cognitive_kernel::{{AsyncObserverAdapter, FileObserver}};");
    println!("   let sync_observer = FileObserver::new(\"sensor.json\");");
    println!("   let async_observer = AsyncObserverAdapter::new(sync_observer);");
    println!("   ```");

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);

    println!("\n=== Demo Complete ===");
    println!("Run with: cargo run --example async_observer_demo --features async");
    println!("For HTTP observer, also enable 'reqwest' feature: --features \"async,reqwest\"");

    Ok(())
}

#[cfg(not(feature = "async"))]
fn main() {
    eprintln!("This example requires the 'async' feature.");
    eprintln!("Run with: cargo run --example async_observer_demo --features async");
    std::process::exit(1);
}
