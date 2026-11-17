//! Async Executor Demo
//!
//! This example demonstrates how to use async executors with the kernel.
//! Requires the `async` feature: `cargo run --example async_executor_demo --features async`
//!
//! The async executors allow non-blocking action execution using tokio's async runtime.

#[cfg(feature = "async")]
use hashbrown::HashMap;
#[cfg(feature = "async")]
use omega_cognitive_kernel::{
    types::Action, AsyncExecutor, AsyncExecutorAdapter, AsyncLoggingExecutor, SandboxedExecutor,
};
#[cfg(feature = "async")]
use tokio::fs;

#[cfg(feature = "async")]
#[tokio::main]
async fn main() -> omega_cognitive_kernel::Result<()> {
    // Initialize tracing for better observability
    tracing_subscriber::fmt::init();

    println!("=== Omega Cognitive Kernel - Async Executor Demo ===\n");

    // Example 1: AsyncExecutorAdapter - wrapping a sync executor
    println!("1. AsyncExecutorAdapter - Wrapping synchronous executor");

    let sync_executor =
        SandboxedExecutor::new(vec!["observe".into(), "stabilize".into(), "explore".into()]);
    let mut async_adapter = AsyncExecutorAdapter::new(sync_executor);

    let actions = vec![
        Action {
            name: "observe".into(),
            params: HashMap::new(),
            constraints_checked: true,
        },
        Action {
            name: "stabilize".into(),
            params: HashMap::new(),
            constraints_checked: true,
        },
    ];

    match async_adapter.execute_async(&actions).await {
        Ok(results) => {
            println!("   ✓ Executed {} actions asynchronously:", results.len());
            for result in &results {
                println!("     - {}", result);
            }
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }

    // Example 2: AsyncLoggingExecutor - native async executor with file logging
    println!("\n2. AsyncLoggingExecutor - Native async executor with file logging");

    let log_path = std::env::temp_dir().join(format!(
        "omega-async-executor-{}.log",
        omega_cognitive_kernel::new_uuid()
    ));
    let mut async_logger =
        AsyncLoggingExecutor::new(vec!["observe".into(), "stabilize".into(), "explore".into()])
            .with_log_file(log_path.clone())
            .echo_stdout(true);

    let actions = vec![
        Action {
            name: "observe".into(),
            params: HashMap::new(),
            constraints_checked: true,
        },
        Action {
            name: "explore".into(),
            params: HashMap::new(),
            constraints_checked: true,
        },
        Action {
            name: "stabilize".into(),
            params: HashMap::new(),
            constraints_checked: true,
        },
    ];

    match async_logger.execute_async(&actions).await {
        Ok(results) => {
            println!("   ✓ Executed {} actions asynchronously:", results.len());
            for result in &results {
                println!(
                    "     - Action: {}, Status: {}",
                    result["action"], result["status"]
                );
            }
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }

    // Show log file contents
    println!("\n   Log file contents:");
    if let Ok(content) = fs::read_to_string(&log_path).await {
        for line in content.lines() {
            println!("     {}", line);
        }
    }

    // Cleanup log file
    let _ = fs::remove_file(&log_path).await;

    // Example 3: Using From trait for easy conversion
    println!("\n3. Using From trait for easy conversion");

    let sync_executor = SandboxedExecutor::new(vec!["test_action".into()]);
    let mut async_executor: AsyncExecutorAdapter<SandboxedExecutor> = sync_executor.into();

    let actions = vec![Action {
        name: "test_action".into(),
        params: HashMap::new(),
        constraints_checked: true,
    }];

    match async_executor.execute_async(&actions).await {
        Ok(results) => {
            println!("   ✓ Converted and executed successfully:");
            for result in &results {
                println!("     - {}", result);
            }
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }

    // Example 4: Custom async executor implementation pattern
    println!("\n4. Custom AsyncExecutor implementation pattern");
    println!("   You can implement AsyncExecutor directly for custom async logic:");
    println!("   ```rust");
    println!("   struct MyAsyncExecutor {{ ... }}");
    println!("   impl AsyncExecutor for MyAsyncExecutor {{");
    println!("       async fn execute_async(&mut self, actions: &[Action]) -> Result<Vec<serde_json::Value>> {{");
    println!("           // Your async logic here");
    println!("           Ok(vec![])");
    println!("       }}");
    println!("   }}");
    println!("   ```");

    println!("\n=== Demo Complete ===");
    println!("Run with: cargo run --example async_executor_demo --features async");

    Ok(())
}

#[cfg(not(feature = "async"))]
fn main() {
    eprintln!("This example requires the 'async' feature.");
    eprintln!("Run with: cargo run --example async_executor_demo --features async");
    std::process::exit(1);
}
