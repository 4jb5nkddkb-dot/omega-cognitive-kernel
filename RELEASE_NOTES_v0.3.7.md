# Omega Cognitive Kernel v0.3.7

**Training-free online strategy-patch runtime** — patch a structured `RuleEffect`, remember it, measure the consequence, roll back if needed.

This is a **research prototype**, not a general agent platform or AGI claim. L2 fixed-point language is heuristic.

---

## Highlights

### L2 biases → DecisionMaker

Meta biases now map onto concrete action scores:

| Bias | Effect |
|------|--------|
| `explore_preference` | Boost explore; mild penalty on exploit / stabilize |
| `self_ref` + `coherence` | Boost stabilize (+ mild observe) |
| `observation_depth` | Boost observe |

Still combined with strategy weights, `RuleEffect`, and `max_risk`.

### Strategy idle decay (observable)

Unused strategies age (`idle_steps`) and slowly lose weight so early lucky arms cannot lock the policy forever.

Bandit task report / CLI `task` now expose:

- `strategy_count`, `idle_strategy_count`
- `mean_strategy_weight`, `mean_idle_weight`, `mean_active_weight`
- `mean_idle_steps`, `max_idle_steps`

### Learner path

L0 owns `Learner::update` (case / strategy / CAS + idle decay). Single ownership of decay.

### Docs for open source

- README and ARCHITECTURE aligned to **0.3.7**
- Clear is / is-not table, safety posture, public vs internal API

---

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.3.7 --locked
```

From source:

```bash
git clone https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel
cd omega-cognitive-kernel
git checkout v0.3.7
cargo test
cargo build --release
```

Binary: `omega-kernel` (also attached as `omega-kernel-linux-x86_64.tar.gz` on the GitHub Release).

---

## Quick check

```bash
omega-kernel demo
omega-kernel run --steps 12
omega-kernel task --episodes 6 --steps 4
omega-kernel status
omega-kernel --help
```

---

## Safety (unchanged invariants)

- L0 action whitelist + constraints
- L1 sandbox: risk / keywords / `RuleEffect` bounds; **no execution of patch source**
- L2 only modulates biases / soft constraints / sanitized soft-nudges
- Checkpoint before accepted patch; rollback + audit log

See [ARCHITECTURE.md](./ARCHITECTURE.md).

---

## Full changelog

See [CHANGELOG.md](./CHANGELOG.md) for 0.3.7 and prior 0.3.x hardening notes.

---

## Compatibility

- Rust: edition 2021, `rust-version = "1.74"`
- Supported integrator surface: `Kernel`, status/run reports, bandit helpers, checkpoint/audit APIs
- `kernel.inner.*` layout is unstable
