---
name: example-safety-probe-debugging
description: Diagnose and repair deterministic Rust example probes for safety gates, arbitration, rollback windows, and other stateful runtime contracts. Use when an example prints failed checks despite exiting successfully, its positive control is invalid, or its inferred cause does not match runtime state transitions.
---

# Example Safety Probe Debugging

Distinguish an invalid probe fixture from a real runtime defect before changing production behavior.

## Workflow

1. **Treat the observable check as the gate.** Read the example's final verdicts and exit status. A zero process status does not override a printed failed assertion. Stop the dependent release or deployment workflow while the result is unresolved.
2. **Trace actual state transitions.** Locate the implementation, result fields, focused tests, and the example's introducing commit. Identify the exact pre-state, action, post-state, timeout/window, and notification contract for each failing case.
3. **Record evidence outside the source tree.** Use a temporary diagnostic consumer of the public API or focused test. Log inputs, relevant state before and after each step, actual returned flags, and causal identifiers. Do not commit ad hoc logs or generated output.
4. **Classify the failure.** Check whether the fixture ever creates the required condition. Compare measured deltas with thresholds, account for filtering/aggregation, and distinguish natural scenario coverage from a controlled positive control. Preserve genuine runtime failures instead of relabeling them as fixture noise.
5. **Repair only the defective layer.** For fixture errors, establish deterministic preconditions, pair each cause with its immediate result, and assert both positive and negative cases. For runtime errors, make the smallest contract-conforming implementation change and add a regression test. Do not remove assertions, weaken thresholds, or skip required checks to obtain a pass.
6. **Make failures observable to automation.** Ensure a probe returns a nonzero status when any declared check fails. Use the actual result/arbitration fields where available rather than inferring them from unrelated state or historical events.
7. **Verify and review.** Run formatting, strict linting, focused regression tests, the complete applicable test suite, and the exact probe command. Repeat stateful probes to check determinism. Inspect the final diff and report root cause, behavior changes, and remaining limitations separately.

## Guardrails

- Do not claim a natural control triggered when its measured trigger count is zero; use an explicit controlled condition and disclose it.
- Do not associate a rollback with an arbitrary earlier change. Pair a pending change with its next consequence and test an expired window independently.
- Do not assume a field populated for producers is also populated by every runtime path; test the consumer-visible contract.
- Keep the original safety objective intact when correcting fixtures or error propagation.
