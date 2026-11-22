# Standard Bandit (UCB) Decision Maker

This is the direct, standard multi-armed bandit implementation for the nonstationary 2-armed task.

## What it does

- Maintains per-arm empirical mean and pull count.
- Selects the arm with the highest UCB1 score: `mean + c * sqrt(ln(t) / n)`.
- Maps arm 0 → `stabilize`, arm 1 → `explore` so the existing reward recording loop continues to work.
- Explores each arm initially by assigning an infinite UCB score when its pull count is zero.

## How to use it

```rust
use omega_cognitive_kernel::api::BanditKernel;
use omega_cognitive_kernel::Kernel;

let mut kernel: BanditKernel = Kernel::with_bandit_ucb("./bandit-state.json");
```

For custom adapters, `Kernel<O, D, E>` accepts the decision maker as the third constructor argument: `Kernel::with_adapters(observer, executor, decision)`. The default `D` remains `ConstrainedDecisionMaker`.

## Feeding rewards back

After each step in the task loop:

```rust
let arm = if action == "explore" { 1 } else { 0 };
kernel.inner.l0.decision.observe(arm, reward); // if using BanditUcbDecisionMaker
```

## Why this exists

`BanditKernel` provides explicit arm tracking and selection instead of relying on soft modulation to change a constrained decision. `DecisionMaker::observe` feeds each selected arm's reward into UCB statistics. L1 and L2 remain independently gated and disabled by default; constructing a `BanditKernel` does not enable either mutation path.
