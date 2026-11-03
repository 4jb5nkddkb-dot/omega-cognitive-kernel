# Omega Cognitive Kernel

**L0 / L1 / L2 metacognitive architecture with `omega = dayan` fixed point**

This repository implements the core closed-loop cognitive architecture discussed in the Omega Core project:

```
观察世界 → 记忆偏置意图 → 预测后决策 → 受限执行 → 结果回写 CAS/策略记忆 → 元评估影响下一轮
```

## Architecture

| Layer | Name | Responsibility |
|-------|------|----------------|
| **L0** | World Interaction | Observe → Predict → Decide → Constrained Execute |
| **L1** | Cognitive Upgrade | Distill → Learn → Reflect → Generate & safely commit cognitive patches |
| **L2** | Metacognition | Monitor metrics → Detect emergence → Enter `observation = observed` mode → Align omega ↔ dayan → Modulate biases & constraints |

### Meta Fixed Point (`omega = dayan`)

When emergence criteria are met, L2 enters self-referential mode. It checks whether the highest attractor (`omega`) and the lowest generative base (`dayan`) converge to the same structure. Successful alignment produces modulation signals that affect the next cycle’s intent bias and safety constraints.

## Quick Start

```bash
cargo run
```

The demo runs several cognitive cycles and prints whether a patch was accepted and whether the system entered the fixed-point mode.

## Project Layout

```
src/
├── main.rs              # Demo entry point
├── lib.rs
├── types.rs             # Core data types
├── error.rs
├── memory/              # Case / Strategy / CAS
├── l0/                  # World interaction layer
├── l1/                  # Cognitive upgrade + sandbox
├── l2/                  # Metacognition + omega=dayan
└── safety/
```

## Safety Notes

- L0 execution is whitelist + constraint checked.
- L1 code patches pass through a sandbox (risk threshold + forbidden keywords).
- Only validated patches are committed to the versioned store.
- L2 never directly mutates code; it only modulates biases and constraints.

## Status

This is a working skeleton focused on clear layering and the metacognitive fixed-point mechanism. Real sensors, world models, and LLM-based patch generation can be plugged in via the existing traits.

---

*Omega Cognitive Kernel — from observation to self-referential coherence.*
