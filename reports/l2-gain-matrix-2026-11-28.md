# L2 gain matrix — v0.7.1 current-tree benchmark

Date: 2026-11-28

Implementation: `79a8861` (tag `v0.7.1`), reproduced on current `origin/main`
`990189f`

Status: **complete; no reproducible L2 gain demonstrated**

This is an unchanged re-execution of the preregistered matrix in
[`docs/EXPERIMENTS.md`](../docs/EXPERIMENTS.md), not a continuation of the stopped
controller-tuning experiments. No seeds, horizon, gates, production defaults, or
primary metric were changed after seeing results. The historical `5078bf3` raw
artifact is unavailable here, so this report does not claim a comparison with
that older source snapshot.

## Fixed design and acceptance rule

- Four scenarios: `stable`, `nonstationary`, `regime_shift`, `cyclic_shift`.
- Four independent arms: L0, L1-only, L2-only, and L1+L2; spectral control is off.
- Discovery seeds `20261115..20261144`; independent validation seeds
  `20270001..20270030`. Each arm runs 12 episodes × 12 steps.
- Total: 240 paired seeds, 960 kernels, and 138,240 decisions.
- Primary contrast: paired raw external reward, **L2-only − L0**, on
  `regime_shift`. Both seed sets require a Student-t 95% lower bound > 0.
- Stable-scenario protection requires a lower bound >= −0.01 in both sets.
- Both primary L2-only arms must contain actually applied committed policies.
  Proposals, commits, cache hits, adjusted reward, and internal error are not
  substitutes for external reward.

Executed command:

```bash
cargo run --locked --release --bin omega-kernel -- benchmark-l2-gain --json \
  > /tmp/omega-benchmark-990189f/l2-gain-matrix.json
```

The current-main run completed in 35.87 seconds including 16.65 seconds of release
compilation; this is fixture throughput, not a production latency benchmark.
Peak reported RSS was 307,396 KiB on the 4-vCPU Linux host. A second execution of
the built binary produced byte-identical JSON. The earlier `v0.7.1` run produced
exactly the same report in 37.33 seconds, so the decision applies to both source
snapshots; this is an exact fixture-result comparison, not a claim of unchanged
production latency or general behavioral equivalence.

## Primary L2-only result

All deltas are external mean-reward differences against L0. `t95` is the paired
Student-t 95% interval; `applied/seed` counts actually attributed L2 policy
execution, not proposals or commits.

| Seed set | Scenario | Reward delta | t95 | Best-arm delta | Applied/seed |
|---|---|---:|---|---:|---:|
| Discovery | `stable` | +0.005556 | [+0.000917, +0.010194] | +0.004630 | 1.33 |
| Discovery | `nonstationary` | −0.016667 | [−0.038852, +0.005519] | −0.029167 | 0.93 |
| Discovery | **`regime_shift`** | **−0.004630** | **[−0.009214, −0.000045]** | −0.005556 | 0.60 (18 total) |
| Discovery | `cyclic_shift` | 0.000000 | [0.000000, 0.000000] | 0.000000 | 0.00 |
| Validation | `stable` | −0.000463 | [−0.002256, +0.001330] | −0.001852 | 1.03 |
| Validation | `nonstationary` | −0.009722 | [−0.022918, +0.003474] | −0.018056 | 0.87 |
| Validation | **`regime_shift`** | **−0.009722** | **[−0.018134, −0.001311]** | −0.011343 | 0.53 (16 total) |
| Validation | `cyclic_shift` | +0.000926 | [−0.000968, +0.002819] | +0.000926 | 0.03 |

For primary `regime_shift`, raw reward changed from 0.792824 to 0.788194 in
discovery and from 0.795139 to 0.785417 in validation (L0 → L2-only). Mean
recovery latency changed from 29.97 to 30.37 steps and from 29.50 to 30.33 steps;
recovery rate was 1.0 in both arms of both sets.

**Decision:** the primary criterion and independent replication both fail;
`reproducible_gain=false`. Both stable safety checks pass. The positive
stable-discovery result and small cyclic-validation result are secondary and
cannot replace the failed primary metric. These bounded results do not establish
that every controller, task, or L2 algorithm is ineffective.

## Full factorial diagnostics

Raw mean reward is listed as `L0 / L1 / L2 / L1+L2`. Each effect includes its
paired Student-t 95% interval; interaction is `L1+L2 − L2 − L1 + L0`.

| Seed set | Scenario | Raw reward by arm | L1 − L0 | Joint − L0 | Interaction |
|---|---|---|---|---|---|
| Discovery | `stable` | 0.791667 / 0.803472 / 0.797222 / 0.793287 | +0.011806 [+0.005748, +0.017863] | +0.001620 [−0.004571, +0.007812] | −0.015741 [−0.023041, −0.008441] |
| Discovery | `nonstationary` | 0.740278 / 0.748380 / 0.723611 / 0.710417 | +0.008102 [−0.007545, +0.023749] | −0.029861 [−0.053123, −0.006599] | −0.021296 [−0.051425, +0.008833] |
| Discovery | `regime_shift` | 0.792824 / 0.795602 / 0.788194 / 0.784259 | +0.002778 [−0.000079, +0.005634] | −0.008565 [−0.016641, −0.000489] | −0.006713 [−0.016892, +0.003466] |
| Discovery | `cyclic_shift` | 0.583565 / 0.582176 / 0.583565 / 0.582639 | −0.001389 [−0.005926, +0.003148] | −0.000926 [−0.004881, +0.003029] | +0.000463 [−0.001049, +0.001975] |
| Validation | `stable` | 0.795139 / 0.796296 / 0.794676 / 0.797685 | +0.001157 [−0.003613, +0.005928] | +0.002546 [−0.001334, +0.006427] | +0.001852 [−0.002911, +0.006615] |
| Validation | `nonstationary` | 0.726157 / 0.733333 / 0.716435 / 0.710648 | +0.007176 [−0.011502, +0.025854] | −0.015509 [−0.035773, +0.004755] | −0.012963 [−0.037895, +0.011969] |
| Validation | `regime_shift` | 0.795139 / 0.790972 / 0.785417 / 0.780556 | −0.004167 [−0.009152, +0.000818] | −0.014583 [−0.023753, −0.005414] | −0.000694 [−0.012995, +0.011606] |
| Validation | `cyclic_shift` | 0.585648 / 0.580556 / 0.586574 / 0.580556 | −0.005093 [−0.011657, +0.001471] | −0.005093 [−0.011657, +0.001471] | −0.000926 [−0.002819, +0.000968] |

No L1 or joint effect satisfies the primary positive-lower-bound criterion in
both seed sets. Interaction signs and intervals do not reproduce consistently;
do not generalize individual scenario intervals into a universal interaction
claim.

## Independent report validation

- Parsed all eight reports and checked 240 complete, ordered seed pairs and 960
  arm reports; expected gate-isolation fields were zero where required.
- Independently recomputed 960 per-seed factorial contrasts and all 32 aggregate
  Student-t intervals from raw per-seed arm rewards.
- Confirmed the complete built-in assessment, actual-intervention preconditions,
  stable safety outcomes, and byte-identical replay.
- At `79a8861`, `cargo test --locked --all-targets` passed, including 217 library
  tests, 3 audit-panel tests, 5 CLI tests, and 9 closed-loop matrix tests;
  `cargo test --locked --doc` passed 1 test.
- After rebasing onto `990189f`, formatting, `cargo clippy --locked --all-targets
  -- -D warnings`, and `cargo test --locked` passed. The latter includes 217
  library, 3 audit-panel, 5 CLI, 11 closed-loop, and 1 doc test.

Artifacts outside the source tree:

- `/tmp/omega-benchmark-990189f/l2-gain-matrix.json` — full 3.1 MiB current-main report.
- `/tmp/omega-benchmark-990189f/l2-gain-matrix-replay.json` — identical replay.
- `/tmp/omega-benchmark-79a8861/l2-gain-matrix.json` — identical `v0.7.1` report.
- `/tmp/omega-benchmark-79a8861/validated-summary.json` — independently checked summary.
- `/tmp/omega-benchmark-990189f/timing.txt` and `SHA256SUMS`.

SHA-256 of each complete pretty-printed JSON report, including final newline:

```text
ec9ba8732a723faa2dc202ed0548ccaf3ba5a454de94bb6b011fc39b177868e4
```

The `/tmp` directory is a local reproducibility aid, not a durable CI artifact.
