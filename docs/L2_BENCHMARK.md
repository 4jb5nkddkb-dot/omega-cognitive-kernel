# L2 Benchmark

## Purpose

`benchmark-l2` compares a constrained-decision baseline with a UCB-based, L2-enabled treatment on the same seeded non-stationary two-armed bandit task. It is intended to turn L2 from a qualitative heuristic into a measurable, repeatable capability.

The benchmark does **not** claim that one run proves improvement. It aggregates multiple seeds and reports treatment deltas against a baseline.

## Run

```bash
cargo run --release --bin omega-kernel -- benchmark-l2
```

Recommended explicit run:

```bash
cargo run --release --bin omega-kernel -- benchmark-l2 \
  --scenario nonstationary \
  --runs 20 \
  --episodes 12 \
  --steps 12 \
  --seed 20261115 \
  --json > l2-benchmark.json
```

Gate sensitivity can be tested without code changes:

```bash
cargo run --release --bin omega-kernel -- benchmark-l2 \
  --scenario nonstationary \
  --commit-threshold 0.55 \
  --commit-cooldown 5 \
  --negative-reward-required 3 \
  --json
```

The three gate parameters are the treatment-branch commit threshold, post-commit cooldown, and consecutive negative-reward requirement. The recommended defaults are `0.55/5/3` for nonstationary, `0.65/8/3` for degraded, and `0.55/5/5` for stable.

The two algorithm branches use identical environment seeds:

- **baseline**: `ConstrainedDecisionMaker` with L2 disabled; the monitor may evaluate metrics, but L2 cannot modify biases, constraints, or `RuleEffect`;
- **treatment**: `BanditKernel` with `BanditUcbDecisionMaker` and L2 enabled, including its detector, fixed-point, modulation, and effect-nudge behavior. The task loop calls `DecisionMaker::observe_with_feedback` after each decision to update UCB statistics.

Both benchmark branches explicitly enable L1 self-modification so the patch/rollback loop is measured. This does **not** change the runtime default `self_modify_enabled = false`.

UCB decisions use hysteresis: a seen candidate cannot replace the incumbent before `min_dwell_steps`, and its UCB must exceed the incumbent by `switch_margin`. An unseen arm can initialize without waiting. An actual safety rollback can force the stabilizing arm. Defaults are `3` and `0.05`; `switch_cost = 0.02` is charged once to a destination arm after switching.

The UCB reward is `external_reward - 0.25 * error_delta - rollback_penalty * rollback_count - switch_cost`. The default rollback penalty is `0.35`. An accepted L1 patch that worsens error by more than `0.05` on its next observation restores the previous `RuleEffect`; only that actual restoration counts as an L1 rollback. Rejected/cooldown-skipped patches are not rollbacks. Restoring an effect may improve later error, but the rollback cost is still charged to the arm that caused it. Configure these values in `[bandit]`; zeros and `min_dwell_steps = 0` restore legacy switching/reward behavior.

This is a composite treatment comparison, not an L2-only ablation: the baseline and treatment use different decision makers. Positive deltas demonstrate measured task improvement for the UCB treatment in this fixture, but cannot be attributed to L2 alone. L2 remains disabled by default.

The short benchmark fixture lowers the treatment `commit_threshold` to `0.55` so the five-signal candidate-to-commit path is exercised. The production/default L2 threshold remains `0.82`; this fixture is for mechanism measurement, not a production configuration recommendation.

Available scenarios:

- `stable`: optimal arm remains stable; tests over-intervention;
- `nonstationary`: optimal arm switches; tests adaptation;
- `degraded`: the environment and prediction signal degrade halfway through; tests consequence feedback and exit behavior.
- `fixedpoint`: holds a low-error coherent window, then injects high error; tests fixed-point enter, dwell, and exit evidence.
- `regime_shift`: arm 0 is initially optimal; at the global midpoint arm 1 becomes optimal and prediction error is forced high (`0.9`, then `0.65` before subsequent steps). The treatment uses a `0.85` reward-mean discount so stale pre-shift estimates do not dominate forever. The fixed baseline keeps its policy.

For `regime_shift`, recovery is the first post-shift step that completes a streak of optimal-arm choices lasting one third of the remaining steps. `recovery_steps` counts from the shift and is one-based. It is `null` when the run never recovers; aggregate `mean_recovery_steps` averages recovered runs only, while `recovery_rate` includes every run.

Each run uses a fresh temporary state path. Persistent benchmark artifacts are removed after the run.

## Metrics

- `mean_reward`: mean reward per bandit decision;
- `best_arm_rate`: fraction of decisions choosing the currently optimal arm;
- `mean_error_delta`: final prediction error minus initial prediction error; lower is better;
- `error_decrease_rate`: fraction of seeded runs where final error is lower than the initial error;
- `mean_patches_accepted`: average accepted L1 patches per run;
- `mean_l2_nudges`: average steps where L2 produced an effect nudge;
- `mean_l2_consequence_score`: short-term reward/error consequence score;
- `mean_l2_proposals` / `mean_l2_commits`: candidate and committed modulation counts;
- `mean_action_switches`: changes of selected action between adjacent decisions;
- `mean_post_commit_action_changes` / `mean_post_commit_arm_changes`: whether the decision immediately after a committed modulation selects a different action or bandit arm;
- `mean_distinct_actions`: action diversity within each run;
- `mean_final_score_breakdown`: weighted fixed-point score components;
- `mean_fixed_point_enters` / `mean_fixed_point_exits`: L2 regime transition counts;
- `mean_max_fixed_point_dwell`: maximum observed fixed-point dwell steps;
- `mean_final_alignment_score`: final L2 alignment score;
- `mean_recovery_steps` / `recovery_rate`: regime-shift recovery latency among recovered runs and fraction recovered;
- `mean_l1_rollback_count`: actual L1 `RuleEffect` restorations after a harmful accepted patch;
- `mean_l2_rollback_count`: actual L2 effect restorations, counted separately;
- `dayan_convergence_rate` / `mean_final_dayan_residual`: fraction with residual at most `0.18`, and mean final residual `abs(1 - candidate_alignment)` from the latest candidate evaluation. The disabled monitor baseline has no candidate, so its residual is `1.0` and convergence is false;
- `mean_dwell_hold_count` / `mean_margin_hold_count`: UCB switch suppressions. `dwell_hold` has priority in the reason field when both gates fail.

The top-level `delta_*` fields are:

```text
treatment aggregate - baseline aggregate
```

## Five-signal score

The current score combines the internal alignment signal with environment and consequence signals:

```text
candidate_score =
    0.45 * alignment
  + 0.30 * reward_change
  + 0.15 * error_change
  + 0.10 * stability
```

`reward_change` requires three consecutive negative reward observations by default, which prevents a single Bernoulli miss from triggering a regime-change response. The `stable` fixture tightens this to five consecutive negatives; `nonstationary` and `degraded` retain three. `error_change` is normalized from the recent prediction-error delta, while `stability` comes from the L2 monitor. The JSON `mean_final_score_breakdown` exposes each weighted contribution.

Fixed-point evidence is intentionally separate from modulation evidence. `candidate_alignment_score` is the raw internal alignment component; `candidate_score` is the weighted score used by the commit gate. A run is counted as entering fixed-point when raw alignment crosses the fixed-point threshold, even if the composite score is below the modulation commit threshold. This proves enter/dwell/exit behavior without conflating it with a policy nudge.

For reward and best-arm rate, positive is better. For error delta, lower is better; the JSON report keeps each branch's value so that interpretation is not hidden.

## Interpretation

A useful treatment result should satisfy all of the following over repeated seeds:

1. reward or best-arm rate improves against baseline;
2. the improvement is not caused only by a higher patch count;
3. error delta does not regress materially;
4. L2 nudges occur in the intended operating context, and fixed-point entries are followed by exits when conditions degrade;
5. results remain directionally stable when the seed range changes.

Do not use `mean_l2_nudges > 0` or `mean_fixed_point_enters > 0` as a success criterion by itself. Those only prove that an L2 path fired, not that the intervention helped.

## Decision replay and status

Every bandit step records `l2_decision` in `trajectory.jsonl`: selected arm/bias, mean reward, exploration bonus, candidate/current UCB scores, reason (`switched`, `dwell_hold`, `margin_hold`, `safety_override`, `explore`, or `exploit`), dwell state, and feedback costs. `replay` displays these fields. `status --json` exposes the latest summary as `last_l2_decision`; it is `null` for a fresh kernel or non-UCB decision maker.

State schema v3 stores these explanations and the UCB statistics/dwell state. Missing fields in v0–v2 snapshots use safe defaults and migrate on load. Older trajectory lines without `l2_decision` also replay with a `null` summary.

## Current treatment finding (2026-11-22)

A local release-build run at the PR #28 baseline used the recommended 20-seed nonstationary configuration (initial seed `20261115`, 12 episodes × 12 steps). It produced these results:

| Metric | Constrained baseline, L2 off | UCB treatment, L2 on | Delta |
|---|---:|---:|---:|
| Mean reward | 0.699306 | 0.746181 | **+0.046875** |
| Best-arm rate | 0.750000 | 0.822222 | **+0.072222** |
| Error-decrease rate | 0.000000 | 0.000000 | 0.000000 |
| Mean action switches | 2.60 | 28.85 | +26.25 |
| Mean L2 commits | 0.00 | 0.85 | +0.85 |
| Mean post-commit arm changes | 0.00 | 0.10 | +0.10 |

The UCB treatment therefore shows a small, measurable positive task-performance delta, and its selected arm can change after a commit. This supersedes the earlier unchanged-reward finding for this configuration. It does not establish statistical significance, show that L2 causes the gain, or demonstrate that prediction error improves. Both branches had an error-decrease rate of zero.

Reproduce the underlying JSON with the recommended command above. Retain the JSON as a release/CI artifact when publishing trend data. Continue to compare other scenarios and seed ranges before making broader effectiveness claims or enabling L2 by default.

## Hardened local run (2026-11-22)

A release run of the hardened implementation with `--runs 20` used the CLI defaults (8 episodes × 8 steps). For the original `nonstationary` scenario, treatment reward was **0.686719** versus baseline **0.697656**, so the new default hysteresis did not improve that fixture. Treatment dwell/margin holds averaged 3.70/7.80. Both branches averaged 2.0 actual L1 rollbacks. This negative result is retained intentionally; it is not evidence that L2 or UCB always improves reward.

With `--scenario regime_shift --runs 20 --episodes 12 --steps 12`, treatment reward was **0.786111** versus fixed baseline **0.552083**, best-arm rate **0.876042** versus **0.500000**, and all 20 treatment runs recovered with mean latency **30.4 steps**. Both branches averaged **2.0 actual L1 rollbacks**. This is still a composite treatment comparison (different decision maker and nonstationary discount), not an L2-only causal ablation or proof of statistical significance.

## CI / regression use

The JSON output supports CI trend tracking and plotting. Keep this benchmark observational rather than enforcing a hard reward threshold until matched-policy L2 ablations and repeated scenarios provide stable evidence. After collecting stable results across several commits, add a regression threshold with an explicit tolerance and record the benchmark parameters in the report.
