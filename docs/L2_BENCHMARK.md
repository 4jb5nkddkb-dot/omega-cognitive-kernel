# L2 Benchmark

## Purpose

`benchmark-l2` compares two `BanditUcbDecisionMaker` kernels on the same seeded non-stationary two-armed bandit task. The only intentional policy difference is L2 enablement. UCB/L1 settings, detector and commit parameters, reward discount, persistence switches, and environment seeds are paired.

The benchmark does **not** claim that one run proves improvement. It aggregates multiple seeds and reports treatment deltas against a baseline. The top-level `paired_runs` array retains both raw `BanditReport` values for each seed, the paired reward/best-arm/error deltas, and `treatment_influenced_l0`. The last flag is true only when an L2 commit, applied policy, or effect nudge occurred; cache hits and proposals alone do not qualify. `l0_influenced_runs` counts these runs. `paired_reward_difference` and `paired_best_arm_difference` report the per-seed mean difference and a 95% Student-t interval (using a normal approximation above 30 degrees of freedom); intervals are `null` with fewer than two runs. These descriptive intervals do not turn non-intervention differences into causal evidence. Use these records to diagnose seed-specific differences before attributing a result to L2.

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

The three gate parameters are applied identically to both branches. The current defaults are `0.35/3/3` for nonstationary and cyclic scenarios, `0.50/3/3` for degraded, and `0.35/3/5` for stable. They are fixture knobs, not production recommendations.

The two algorithm branches use identical environment seeds and `BanditUcbDecisionMaker` policies:

- **baseline**: L2 interventions disabled; monitoring, fixed-point scoring, and residual observability continue;
- **treatment**: the same UCB/L1 configuration with resident L2 enabled. Proposals are separate from executable commits, and commits are separate from the immutable policy applied to the current L0 action.

Both benchmark branches explicitly enable L1 self-modification so the patch/rollback loop is measured. This does **not** change the runtime default `self_modify_enabled = false`.

UCB decisions use hysteresis: a seen candidate cannot replace the incumbent before `min_dwell_steps`, and its UCB must exceed the incumbent by `switch_margin`. An unseen arm can initialize without waiting. An actual safety rollback can force the stabilizing arm. Defaults are `3` and `0.05`; `switch_cost = 0.02` is charged once to a destination arm after switching.

The UCB reward is `external_reward - 0.25 * error_delta - rollback_penalty * rollback_count - switch_cost`. The default rollback penalty is `0.35`. An accepted L1 patch that worsens error by more than `0.05` on its next observation restores the previous `RuleEffect`; only that actual restoration counts as an L1 rollback. Rejected/cooldown-skipped patches are not rollbacks. Restoring an effect may improve later error, but the rollback cost is still charged to the arm that caused it. Configure these values in `[bandit]`; zeros and `min_dwell_steps = 0` restore legacy switching/reward behavior.

This is an L2-only ablation under the fixture's paired UCB policy, not a comparison of different decision makers. Positive deltas would support an L2 effect in this fixture; they would not establish external validity or statistical significance. New kernels enable L2 by default; the baseline explicitly disables it for comparison.

The short benchmark fixture lowers both branches' `commit_threshold` to `0.35` (`0.50` for `degraded`) to exercise the controller. The production/default threshold remains `0.82`. This fixture is for mechanism measurement, not a production configuration recommendation.

Available scenarios:

- `stable`: optimal arm remains stable; tests over-intervention;
- `nonstationary`: optimal arm switches; tests adaptation;
- `degraded`: the environment and prediction signal degrade halfway through; tests consequence feedback and exit behavior.
- `fixedpoint`: holds a low-error coherent window, then injects high error; tests fixed-point enter, dwell, and exit evidence.
- `regime_shift`: arm 0 is initially optimal; at the global midpoint arm 1 becomes optimal and prediction error is forced high (`0.9`, then `0.65` before subsequent steps). Both branches use a `0.85` reward-mean discount.
- `cyclic_shift`: the optimal arm alternates every 12 global steps. Early steps in each cycle inject the same observation error in both branches.

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
- `dayan_convergence_rate` / `mean_final_dayan_residual`: internal relaxation convergence, measured as the final residual being at most `0.18`, below its initial value, with at least one decreasing update. This is separate from external optimal-arm recovery. For a frozen target, `r+ = (1 - relaxation)r`; moving targets add their absolute target change, so no global contraction claim is made.
- `mean_committed_policies_applied`: committed snapshots that actually governed an L0 action, as distinct from proposals or queued commits;
- `mean_dwell_hold_count` / `mean_margin_hold_count`: UCB switch suppressions. `dwell_hold` has priority in the reason field when both gates fail.
- `mean_lyapunov_vetoes`: proposals rejected by the one-step residual-potential check;
- `mean_meta_cache_hits`: context-cache lookups, not evidence of improved recovery;
- `recovery_event_rate` / `mean_recovery_steps`: external optimal-arm recovery after a shift;
- `mean_warm_recovery_steps`: latency only when a cache hit produces a commit, that exact commit executes within the recovery window, and the required optimal-arm streak completes. It is `null` when there are no such events. A cache hit alone never qualifies.

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

State schema v5 stores the resident controller, online Dayan state, bounded cache, validated pending/applied policy snapshots, UCB statistics, and decision explanations. Missing legacy L2 permissions default to `false`; malformed or noncommitted snapshots collapse to `KEEP_L0`. Older trajectory lines without the newer fields replay with safe defaults.

## Historical results

The earlier composite constrained-baseline/UCB-treatment results are not evidence for this controller: they changed the decision maker as well as L2. They are intentionally omitted rather than presented as current acceptance evidence.

## Current paired result (2026-11-23)

A release-build run used 20 seeds beginning at `20261115`, with 12 episodes × 12 steps, for `nonstationary`, `regime_shift`, and `cyclic_shift`. All three completed successfully.

| Scenario | Reward, off → on | Best-arm rate, off → on | Commits / applied | Cache hits | Warm recovery |
|---|---:|---:|---:|---:|---:|
| `nonstationary` | 0.739931 → 0.740625 | 0.802083 → 0.803125 | 0.00 / 0.00 | 6.95 | `null` |
| `regime_shift` | 0.787153 → 0.787153 | 0.877431 → 0.877431 | 0.00 / 0.00 | 12.85 | `null` |
| `cyclic_shift` | 0.584028 → 0.584028 | 0.526736 → 0.526736 | 0.00 / 0.00 | 10.00 | `null` |

The regime and cyclic runs recovered with the same mean external latency in both branches (29.85 and 6.05 steps respectively). Proposals were vetoed by the Lyapunov gate; there were no executable commits. Cache hits therefore provide **no evidence of warm-start benefit**. The tiny nonstationary reward difference is not accompanied by L0 policy intervention and is not an L2 effectiveness claim. Prediction-error improvement also remains unproven. In the current per-seed report, only seed `20261120` has a nonzero reward delta (`0.01388888888888884`), while all 20 runs have `treatment_influenced_l0 = false`.

Reproduce with the recommended command, replacing `nonstationary` with `regime_shift` and `cyclic_shift` for the other two required runs. Retain the JSON as a CI artifact when publishing trend data. Continue to compare other seed ranges before making broader effectiveness claims or enabling L2 by default.

## CI / regression use

The JSON output supports CI trend tracking and plotting. Keep this benchmark observational rather than enforcing a hard reward threshold until matched-policy L2 ablations and repeated scenarios provide stable evidence. After collecting stable results across several commits, add a regression threshold with an explicit tolerance and record the benchmark parameters in the report.
