"""Demonstrate cross-process persistence, step_id idempotency, and AuditLog replay.

Run after `pip install omega-cognitive-kernel` (or `maturin develop`):
    python examples/python/service_policy_adapter.py
"""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

import omega_cognitive_kernel_python as ok


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="omega-service-adapter-") as tmp:
        state_path = str(Path(tmp) / "service_state.json")

        # Process A: emits a strategy patch and persists ExternalLoopState
        proc_a = ok.ClosedLoopSession.from_preset("control_plane", state_path)
        d1 = json.loads(proc_a.step_external_json(1, 0.60, 0.20))
        print("Process A step 1 patch:", d1["rule_patch"])

        # Process B: restarts from the same state file
        proc_b = ok.ClosedLoopSession.from_preset("control_plane", state_path)

        # Replaying step_id=1 is idempotently ignored across restarts
        dup = json.loads(proc_b.step_external_json(1, 0.75, 0.15))
        print("Process B duplicate step 1 keep_steady:", dup["keep_steady"])

        # Reporting degraded feedback rolls back to ckpt-1 with cooldown backoff
        rolled = json.loads(proc_b.step_external_json(2, 0.65, 0.20, None, "degraded"))
        print("Process B step 2 rollback_to_checkpoint:", rolled["rollback_to_checkpoint"])

        audit = ok.AuditLog.beside_state(state_path)
        print("Audit summary:", audit.summarize())


if __name__ == "__main__":
    main()
