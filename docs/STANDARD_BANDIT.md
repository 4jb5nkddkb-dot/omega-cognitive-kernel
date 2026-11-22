# Standard Bandit (UCB) Decision Maker

This is the direct, standard multi-armed bandit implementation for the nonstationary 2-armed task.

## What it does

- Maintains per-arm empirical mean and pull count.
- Selects the arm with the highest UCB1 score: `mean + c * sqrt(ln(t) / n)`.
- Maps arm 0 → `stabilize`, arm 1 → `explore` so the existing reward recording loop continues to work.
- Forces initial exploration (infinite UCB when count = 0).

## How to use it

```rust
use omega_cognitive_kernel::l0::bandit_decision::BanditUcbDecisionMaker;
use omega_cognitive_kernel::l0::observer::MultiModalObserver;
use omega_cognitive_kernel::l0::executor::SandboxedExecutor;
use omega_cognitive_kernel::Kernel;

// Construct a kernel whose decision maker is the standard UCB.
let decision = BanditUcbDecisionMaker::default();
let mut kernel = Kernel::with_adapters(
    MultiModalObserver::new(),
    SandboxedExecutor::new(vec!["observe".into(), "explore".into(), "exploit".into(), "stabilize".into()]),
);
// Note: to swap the decision maker you currently need to construct via the generic path
// or replace `inner.l0.decision` after creation if the types match.
// For the benchmark, the simplest path is to keep the existing ConstrainedDecisionMaker
// and let L2 (or a future adapter) drive exploration, OR construct a fully typed Kernel
// with BanditUcbDecisionMaker as the D parameter.
```

## Feeding rewards back

After each step in the task loop:

```rust
let arm = if action == "explore" { 1 } else { 0 };
kernel.inner.l0.decision.observe(arm, reward); // if using BanditUcbDecisionMaker
```

## Why this exists

The previous L2 soft-modulation path could commit but never changed the chosen arm (post_commit_arm_changes = 0). This is the standard, explicit UCB solution that actually tracks and selects arms.
