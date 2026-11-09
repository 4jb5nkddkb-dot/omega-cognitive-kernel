# Omega Cognitive Kernel v0.3.8

**Training-free online strategy-patch runtime** — now with a public adapter surface for real-world observe/act injection.

This is still a **research prototype**, not a general agent platform. L2 fixed-point language remains heuristic.

---

## Highlights

### Public adapters

- `Observer` / `Executor` re-exported from the crate root
- `Kernel` is generic over observer + executor; **defaults unchanged**
- `Kernel::with_adapters` / `with_adapters_and_path` for injection
- Built-in first adapters: `FileObserver`, `LoggingExecutor`
- Example: `cargo run --example custom_adapters`

### Product README

Lead-in rewritten for problem/value clarity while keeping technical accuracy and safety posture.

---

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.3.8 --locked
```

From source:

```bash
git clone https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel
cd omega-cognitive-kernel
git checkout v0.3.8
cargo test
cargo build --release
```

Binary: `omega-kernel` (also attached as `omega-kernel-linux-x86_64.tar.gz` when the release workflow completes).

---

## Quick check

```bash
omega-kernel demo
omega-kernel run --steps 12
omega-kernel task --episodes 6 --steps 4
cargo run --example custom_adapters
```

---

## Compatibility

- Default `Kernel::new` / `open` / CLI / bandit paths unchanged
- New API is additive for integrators who implement the traits
- Supported surface: `Kernel`, status/run reports, bandit helpers, checkpoint/audit, `Observer` / `Executor`

See [CHANGELOG.md](./CHANGELOG.md) and [ARCHITECTURE.md](./ARCHITECTURE.md).
