"""Guard an LLM agent tool-calling loop with Omega's `control_plane` preset.

Run after `pip install omega-cognitive-kernel` (or `maturin develop`):
    python examples/python/llm_agent_guard.py
"""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

import omega_cognitive_kernel_python as ok


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="omega-llm-guard-") as tmp:
        state_path = str(Path(tmp) / "agent_state.json")
        session = ok.ClosedLoopSession.from_preset("control_plane", state_path)

        # (step_id, tool_error_rate, action_risk, previous_outcome)
        workload = [
            (1, 0.05, 0.10, None),        # Steady state: L2 blocks L1 writes
            (2, 0.58, 0.22, None),        # Tool errors spike: L1 emits patch
            (3, 0.64, 0.25, "degraded"),  # Patch degraded: trigger rollback + backoff
            (4, 0.11, 0.12, "improved"),  # Recovered: L2 re-enters steady gate
        ]

        policy_weight = 1.0
        checkpoints: dict[str, float] = {}

        for step_id, err, risk, outcome in workload:
            directive = json.loads(
                session.step_external_json(step_id, err, risk, None, outcome)
            )
            rollback_id = directive.get("rollback_to_checkpoint")
            if rollback_id and rollback_id in checkpoints:
                policy_weight = checkpoints[rollback_id]

            patch = directive.get("rule_patch")
            if patch and not patch["shadow_only"]:
                checkpoints[f"ckpt-{step_id}"] = policy_weight
                policy_weight = max(0.1, policy_weight + patch["delta_weight"])

            l2 = directive.get("l2_prior") or {}
            print(
                f"step={step_id} gate={directive['l1_gate']} "
                f"weight={policy_weight:.3f} rollback={rollback_id} "
                f"lyapunov={l2.get('lyapunov_potential', 0.0):.4f} "
                f"expert={l2.get('selected_expert', '')}"
            )


if __name__ == "__main__":
    main()
