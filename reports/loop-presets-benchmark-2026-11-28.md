# 7-Mode Closed-Loop Preset Benchmark & Selection Protocol (2026-11-28)

**Source baseline:** `v0.8.0` (`omega-kernel benchmark-loops`, `tests/closed_loop_matrix.rs`).

## Reproducible Commands

Run the built-in deterministic benchmark suite on your target hardware to measure machine-specific `avg_step_us` latency alongside error, patch, rollback, and steady-state counts:

```bash
# Steady low-error workload (verifies KEEP_L0 and L2 write suppression)
omega-kernel benchmark-loops --scenario steady --steps 24 --json

# Regime-shift workload (steady -> error spike + rollback -> recovery)
omega-kernel benchmark-loops --scenario shift --steps 24 --json

# High-frequency noisy workload with risk spikes (verifies CooldownTracker backoff)
omega-kernel benchmark-loops --scenario noisy --steps 24 --json
```

> **Note on latency measurements:** Step latency (`avg_step_us`) depends on host CPU, build profile (`--release` vs. debug), and OS timer resolution. Always collect latency numbers on your deployment host using `--release` builds rather than relying on cross-machine estimates.

## Verified Behavioral Matrix Across All 7 Presets

All behavioral invariants below are continuously enforced in CI by `tests/closed_loop_matrix.rs` (14 integration and stress tests):

| Preset (`CLI` / `Python`) | `(L0, L1, L2)` | Execution Mode | Steady-State Behavior | Rollback & Anti-Oscillation Guard | Recommended Real-World Use Case |
| :--- | :---: | :---: | :--- | :--- | :--- |
| `deterministic` | `(1, 0, 0)` | Internal | Strict `KEEP_L0` (`l1_gate = disabled`) | Zero strategy mutation | Baseline control group / deterministic edge runtime |
| `hot-patcher` / `hot_patcher` | `(0, 1, 0)` | External | Active when `error >= 0.30` and `risk <= 0.65` | `CooldownTracker` exponential rollback backoff + non-finite input rejection | External service hot-patcher without meta-layer overhead |
| `meta-guard` / `meta_guard` | `(0, 0, 1)` | External | Real `L2MetaLayer` prior (`l1_gate = disabled`) | Read-only immutable `L2PriorPayload` (`kl_prior_bias`, `lyapunov_potential`) | Macro regime supervisor / safety monitor for external systems |
| `fast-adapt` / `fast_adapt` | `(1, 1, 0)` | Internal | L0 loop with direct L1 patching | `min_steps_between_patches` + checkpoint rollback | Lightweight local fast-adaptation experiments |
| `steady-prior` / `steady_prior` | `(1, 0, 1)` | Internal | Default safe mode (`l1_gate = disabled`) | Bounded L2 modulation + fixed-point suppression | Production default for stable adaptive agents |
| `control-plane` / `control_plane` | `(0, 1, 1)` | External | L2 sets `blocked_by_l2` when steady | L2 arbitration + `CooldownTracker` + cross-process `ExternalLoopState` | External LLM agent / microservice policy control plane |
| `full-kernel` / `full_kernel` | `(1, 1, 1)` | Internal | Full `L0 -> L1 -> L2 -> bias next L0` loop | L2 write arbitration (`blocks_l1_write`) preserving static `LayerSwitches` | Autonomous end-to-end adaptive kernel |

## Runnable Integration Examples

- Python LLM agent guard: [`examples/python/llm_agent_guard.py`](../examples/python/llm_agent_guard.py)
- Python cross-process service adapter: [`examples/python/service_policy_adapter.py`](../examples/python/service_policy_adapter.py)
- Rust external control plane: [`examples/external_control_plane.rs`](../examples/external_control_plane.rs)
