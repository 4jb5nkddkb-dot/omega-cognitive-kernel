# Omega Cognitive Kernel

**Training-free online strategy adaptation** — patch a structured `RuleEffect`, measure the consequence, roll back if needed.

> When error rises, the kernel emits a bounded policy patch, persists it as cognitive state, and changes the next cycle’s decisions.  
> **No weight updates. No code execution. Full audit trail.**

```
World (L0) → Reflect & patch (L1) → Meta-modulate (L2) → bias next L0
```

## Why this exists

Most adaptive systems either:
- require expensive retraining / fine-tuning, or
- allow arbitrary code changes that are hard to audit and reverse.

Omega takes a third path: **structured, reversible policy patches**.

It is designed for systems that need to adapt *while running*, under tight safety and audit requirements.

Classical control / RL supply the skeleton (observe → predict → decide → act). The engineering bet is different from fine-tuning or PPO: **patch the policy surface, remember the effect, measure the consequence, roll back if needed**.

| Doc | Topic |
|-----|--------|
| **[docs/PRODUCT.md](docs/PRODUCT.md)** | Positioning & monetization posture |
| **[docs/INTEGRATING.md](docs/INTEGRATING.md)** | Embed + adapters + config |
| **[docs/INBOX_VERTICAL.md](docs/INBOX_VERTICAL.md)** | **Inbox vertical demo** (event queue + policy gate + audit) |
| **[docs/API_FREEZE_0.4.md](docs/API_FREEZE_0.4.md)** | Integrator API freeze (**in effect as of v0.4.0**) |

---

## What this is / is not

| Is | Is not |
|----|--------|
| Structured online patches (`RuleEffect`: damping, action preference, explore/risk) | Hot-loading or executing arbitrary new source code |
| Durable cognitive state in memory + `state.json` | Full Bayesian filter / industrial PID / AGI |
| Auditable, checkpointable, reversible adaptation | “Emergence” as a proven fixed point |
| Minimal runtime (JSON state + small Kernel API) | A substitute for large-scale model training |
| L2 **heuristic** meta-modulation of biases & soft constraints | A license to bypass L0/L1 safety |

**Status:** research prototype / experimental runtime (**v0.4.0**). **Integrator API frozen for 0.4.x** — see freeze doc. Internals (`kernel.inner`, heuristics) may still evolve.

---

## Install

**From Git (recommended)**

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --locked
# pin a release:
# cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.4.0 --locked
```

**From source**

```bash
git clone https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel
cd omega-cognitive-kernel
cargo test
cargo build --release
# binary: target/release/omega-kernel
```

Binary name: `omega-kernel`. Linux tarball also attached on [GitHub Releases](https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel/releases).

---

## Quick start

```bash
cargo run -- demo
cargo run -- run --steps 20
cargo run -- status
cargo run -- --config examples/omega.example.toml status
cargo run -- task --episodes 6 --steps 4
cargo run -- audit --tail 30
cargo run -- checkpoints
cargo run -- checkpoint --label before-x
cargo run -- rollback <UUID>
cargo run -- --state ./my-agent.json run --steps 10
cargo run -- reset --yes
```

### Product demo: Inbox vertical (recommended)

Pluggable **observe / execute**, scripted text-event stream, optional `RuleEffect` under load, **audit.jsonl** + **actions.log**:

```bash
cargo run --example inbox_vertical

# keep workspace on disk for inspection with a teammate
KEEP_INBOX=1 cargo run --example inbox_vertical
```

Details: **[docs/INBOX_VERTICAL.md](docs/INBOX_VERTICAL.md)**.  
Directory-based variant: `cargo run --example drop_inbox_loop`.

### What `demo` proves

1. Kernel starts with relatively high uncertainty.
2. High error can trigger L1 patches that carry a **`RuleEffect`** (not code `eval`).
3. Effect changes error damping, action preference, `explore_bonus`, and `risk_tolerance`.
4. Report shows early vs final `avg_err` and whether error decreased.
5. Optional L2 soft-nudges and fixed-point enter/exit are visible in step logs.

### What `task` (bandit) proves

- Non-stationary 2-armed bandit closes the reward → strategy-memory loop.
- Report includes **strategy-decay metrics** (idle strategies, mean/max idle steps, active vs idle weights).

---

## Library API

```rust
use omega_cognitive_kernel::Kernel;

fn main() -> omega_cognitive_kernel::Result<()> {
    let mut kernel = Kernel::open(None)?;
    let report = kernel.run("maintain coherent self-model", 10)?;
    println!(
        "err {:.3} → {:.3} (decreased={})",
        report.early_avg_err, report.final_avg_err, report.error_decreased
    );
    let st = kernel.status();
    println!(
        "L2 in_fp={} dwell={} align={:.3}",
        st.in_fixed_point, st.dwell_steps, st.last_alignment_score
    );
    let _ckpt = kernel.checkpoint("stable")?;
    kernel.save()?;
    Ok(())
}
```

### Custom adapters (observe / act)

Implement `Observer` and `Executor`, then inject them:

```rust
use omega_cognitive_kernel::{FileObserver, Kernel, LoggingExecutor};

let observer = FileObserver::new("/tmp/status.json");
let executor = LoggingExecutor::new([
    "noop".into(), "observe".into(), "explore".into(),
    "exploit".into(), "stabilize".into(),
]).with_log_file("/tmp/actions.log");

let mut kernel = Kernel::with_adapters(observer, executor);
let report = kernel.run("adapt to external signal", 8)?;
```

Built-ins: `DirSnapshotObserver`, `HttpObserver`, `FileObserver`, `LoggingExecutor`.  
End-to-end stories: `examples/inbox_vertical.rs`, `examples/drop_inbox_loop.rs`, `examples/config_and_dir.rs`, `examples/custom_adapters.rs`.

Supported surface: `Kernel`, `KernelConfig`, `KernelStatus`, `RunReport`, task helpers, checkpoint/audit APIs, `Observer` / `Executor`. Prefer these over reaching into `kernel.inner` in downstream code.

---

## Architecture (short)

| Layer | Role |
|-------|------|
| **L0** | Observe → predict → decide (strategy + `RuleEffect` + **L2 biases**) → sandboxed execute → **Learner** (case/strategy/CAS + idle decay) |
| **L1** | Distill → reflect (strategy/error context + **cooldown**) → validate/sanitize → commit **`RuleEffect`** |
| **L2** | Metrics → optional fixed-point entry/dwell/exit → modulate biases/constraints + **soft `RuleEffect` nudge** |

See **[ARCHITECTURE.md](ARCHITECTURE.md)** for data flow, `RuleEffect` contract, L2 mapping, and safety table.

### Safety posture

- **L0:** action whitelist + constraints (`max_risk` affects scoring)
- **L1:** risk + keywords + **`RuleEffect` bounds**; runtime applies structured effects only; patch cooldown
- **L2:** biases & soft constraints only — no actions, no CAS writes
- **Checkpoints** (FIFO) before accepted patches; **rollback** restores snapshot; **audit** log per step

### Persistence & audit

```
.omega/
  state.json
  audit.jsonl
  checkpoints/
```

---

## Design stance

**Mature ideas, unusual packaging.** The distinctive choice is a **training-free patch loop**: typed effect, durable memory, audit + checkpoint, optional meta soft-nudge.

Strategy memory ages unused arms (`idle_steps`) and applies **idle decay** so early lucky weights cannot lock the policy forever. Bandit reports expose that decay for inspection.

---

## Status

| Stage | Meaning |
|-------|---------|
| **S1–S3 / Hardening** | Consequence, persist, audit, bandit, bounds |
| **0.3.7–0.3.9** | L2→decision, adapters, config, Dir/Http, drop-inbox |
| **0.4.0** | **Integrator API freeze** for 0.4.x |
| **Inbox vertical** | `inbox_vertical` example + [docs/INBOX_VERTICAL.md](docs/INBOX_VERTICAL.md) |
| **Next** | More verticals, optional crates.io, commercial dashboard candidate |

See [CHANGELOG.md](CHANGELOG.md). Release notes: [RELEASING.md](RELEASING.md) if present.

---

## Contributing

Issues and PRs are welcome. Please:

1. Keep the safety invariants (no arbitrary code execution path; `RuleEffect` remains the only runtime policy surface).
2. Add or extend tests for behavioral changes (`cargo test`).
3. Update CHANGELOG for user-visible changes.
4. Avoid breaking the [0.4 freeze list](docs/API_FREEZE_0.4.md) without a major bump discussion.

---

## License

MIT — see [LICENSE](LICENSE).

---

*Omega Cognitive Kernel — patch, remember, measure, roll back.*
