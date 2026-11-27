---
name: paired-factorial-benchmarks
description: Design and validate paired factorial benchmarks for independently gated system components. Use when measuring the gains, costs, or interactions of feature switches such as L1/L2, with preregistered metrics, deterministic replay, and an independent validation seed set.
---

# Paired Factorial Benchmarks

Separate main effects and interactions without confusing component activity with task improvement.

## Workflow

1. **Inspect the intervention path.** Trace each gate to the decision and external outcome. Identify arbitration between components and whether an internal metric can change decisions directly or only through shaped feedback. Do not promise a positive effect before this inspection.
2. **Preregister before the discovery run.** Record the implementation baseline, arms, scenarios, seed ranges, horizon, common configuration, primary metric, success and safety thresholds, and stopping rule. Keep a small mechanism preflight separate from effectiveness evidence. Reserve a disjoint validation seed set.
3. **Use a full factorial for two independent gates.** Run `00`, `10`, `01`, and `11` from equivalent initial states. For a metric `Y`, compute per seed:
   ```text
   first-only  = Y10 - Y00
   second-only = Y01 - Y00
   joint       = Y11 - Y00
   interaction = Y11 - Y01 - Y10 + Y00
   ```
   Never infer an interaction from only the joint arm.
4. **Align environment randomness.** Give every arm the same environment seed and consume random numbers by environment position, not by the arm's action history. Use common random numbers to compare an action-dependent success probability at the same position. Verify byte-stable report replay where the implementation supports it.
5. **Separate outcome and mechanism.** Use external task reward or an equivalent task outcome as primary. Report internal prediction error and adjusted/shaped reward separately. Record accepted changes, vetoes, actual execution attribution, arbitration, rollback, and switching costs as diagnostics rather than success criteria.
6. **Test the measurement contract.** Cover gate isolation, four-arm contrast formulas, finite metrics, and deterministic replay. Reuse the existing test and CLI patterns; avoid changing production defaults or state migration solely for an experiment.
7. **Run the fixed matrix.** Execute preflight, discovery, and validation in release mode where appropriate. Store large machine-readable reports outside the source tree, include commands and checksums, and retain negative results. Do not tune thresholds or change the primary metric after seeing outcomes.
8. **Apply the preregistered decision.** Require the specified discovery criterion and independent validation before calling an effect reproducible. A descriptive interval is not universal causal evidence. Stop or write a new experiment card when the current hypothesis fails; do not continue tuning within the same evaluation.

## Verification and delivery

- Run the repository's focused tests, full applicable test suite, formatting, and lint/MSRV checks.
- Parse generated JSON, assert expected arm counts and gate isolation, and independently recompute representative contrasts.
- Compare an existing benchmark with its unchanged baseline when shared task code changes; distinguish additive report fields from behavior changes.
- Document exact executed commands, sample sizes, outcomes, evidence limits, and artifact location. Temporary files are not durable CI artifacts.
- Do not present patch counts, commit counts, cache hits, or lower internal error as proof of external benefit.
