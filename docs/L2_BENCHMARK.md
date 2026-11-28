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

## L1/L2 factorial benchmark

Use `benchmark-l1l2` to separate the two independent mutation gates instead of
inferring L1's effect from the existing L2-only ablation. The arm order in JSON
is `l0`, `l1`, `l2`, `l1_l2`: both disabled, L1 only, L2 only, and both enabled.
All four arms share the same initial UCB state and per-position environment RNG.
The fixture explicitly disables spectral control and uses the same resident-L2
thresholds as `benchmark-l2`.

```bash
cargo run --locked --release --bin omega-kernel -- benchmark-l1l2 \
  --scenario regime_shift --runs 30 --episodes 12 --steps 12 \
  --seed 20261115 --json > discovery-regime_shift.json
```

Run the same command for `stable`, `nonstationary`, and `cyclic_shift`; use
`20270001` for the independent validation set. The report contains every arm's
per-seed `BanditReport`, aggregate metrics, and four paired contrasts:

```text
l1_effect         = L1 - L0
l2_effect         = L2 - L0
joint_effect      = L1+L2 - L0
interaction_effect = L1+L2 - L2 - L1 + L0
```

Each contrast includes Student-t 95% intervals for external reward, bootstrap
95% intervals for reward, and paired best-arm and adjusted-reward differences.
`mean_adjusted_reward` is the actual UCB learning feedback after error,
rollback, and switching costs; it is not external task reward. Patch counts,
`l1_blocked_by_l2_count`, commits, and applied policies are mechanism metrics,
not success criteria. The completed preregistered matrix and its evidence
limits are recorded in `docs/EXPERIMENTS.md`; the large JSON artifacts are not
committed to the source tree.

## Preregistered L2 gain matrix

`benchmark-l2-gain` runs the complete four-scenario factorial design for both
fixed seed sets. Its primary contrast is **L2-only minus L0**, using raw external
reward rather than adjusted learning feedback. The default run executes 960
independent kernels and 138,240 decisions:

```bash
cargo run --locked --release --bin omega-kernel -- benchmark-l2-gain --json \
  > /tmp/l2-gain-matrix.json
```

The fixed scenarios are `stable`, `nonstationary`, `regime_shift`, and
`cyclic_shift`, with 30 paired seeds per scenario in each set. Discovery starts
at `20261115`; validation starts at `20270001`. Every arm runs `12 × 12` steps.
The command does not tune controller settings or change production defaults.

`assessment.reproducible_gain` is true only when all preregistered conditions
hold: positive Student-t 95% lower bounds in both `regime_shift` seed sets,
`stable` lower bounds no worse than `-0.01`, and at least one actually applied
L2 policy in each primary seed set. A discovery-only effect, proposal, commit,
cache hit, internal-error improvement, or adjusted-reward improvement does not
qualify. Missing primary reports and underpowered one-seed runs are rejected.

For a fast mechanism preflight, use `--smoke --json`. It runs two seeds per set
with an `8 × 8` horizon and is not effectiveness evidence. CI stores this report
as `l2-gain-smoke`; the full large JSON remains outside the source tree.

The completed 2026-11-27 default matrix did **not** demonstrate a reproducible
L2 gain. Discovery `regime_shift` reward difference was `-0.004630`, t95
`[-0.009214, -0.000045]`; validation was `-0.008333`, t95
`[-0.014949, -0.001718]`. Both primary sets had actually applied interventions
and both stable safety checks passed. These findings are bounded to this UCB
fixture and seed matrix. Exact commands, other scenarios, and the artifact
checksum are in `docs/EXPERIMENTS.md`.

The unchanged matrix was rerun on `v0.7.1` (`79a8861`) on 2026-11-28, then
reproduced byte-for-byte on current `origin/main` (`990189f`). Discovery
`regime_shift` remained `-0.004630`; validation was `-0.009722`, t95
`[-0.018134, -0.001311]`. The primary and replication criteria failed again;
both stable safety checks passed. The current-tree report, complete four-arm
summary, verification, and decision are in
[`reports/l2-gain-matrix-2026-11-28.md`](../reports/l2-gain-matrix-2026-11-28.md).
No controller defaults were changed by this measurement. Risk-sensitive
deployments can explicitly disable L2 and use the L0 baseline; the fixture does
not justify a universal claim of harm or an implicit global default change.

## Spectral research experiment (default off)

The ordinary benchmark and new kernels keep the resident L2 behavior described above,
but **spectral intervention is disabled by default**. `L2Policy::spectral_weighting`
and legacy snapshot fields also default to false. Spectral control, its gate/history,
and its experiment settings are not persisted as an enabled production permission;
restart therefore does not enable this research intervention. A valid persisted
`spectral_weighting` flag is retained as policy data, not as a new experiment opt-in.

Enable it only for a paired research run:

```bash
cargo run --locked --release --bin omega-kernel -- benchmark-l2 \
  --scenario regime_shift --runs 30 --episodes 12 --steps 12 \
  --seed 20261115 --spectral-control --diagnostics --json \
  > discovery-regime_shift.json
```

Run the same command for `stable`, `nonstationary`, and `cyclic_shift`. For the
independent validation set, change `--seed` to `20270001`. `--diagnostics` includes
per-step treatment records without enabling spectral control. The ablation flags
`--no-purity-prior`, `--no-reverse-brake`, and `--no-history-discount` require
`--spectral-control`; without that master flag, all experimental components are off.
JSON reports sort score-breakdown keys, use deterministic aggregation, and support
byte-for-byte replay comparisons without float tolerances.

The preregistered 2026-11-25 discovery run produced a `regime_shift` paired reward
difference of **-0.003935185185185179**, Student-t 95% interval
**[-0.007031866098688526, -0.000838504271681832]**, and bootstrap 95% interval
**[-0.007175925925925919, -0.001388888888888884]**. The independent validation
run had zero reward difference and zero-width intervals. Stable-scenario differences
were zero in both sets.

The reports were regenerated on `main` at **`9b5e13a`** using the corrected
pre-action dry-run. In 4320 discovery `regime_shift` dry-runs, **19** changed the
candidate arm, **9** changed the counterfactual selected arm, and **10** changed the
candidate without changing the selected arm. Seven of 30 seeds met the formal L2
execution-attribution condition; no history discount fired. The earlier post-action
dry-run record of 9 candidate changes and 0 selected changes is not a measurement of
the current implementation. This **negative result stopped the spectral-control route**:
some interventions changed actions, but the primary reward criterion failed. It is not
evidence of a validated control improvement or a reason to enable the feature by default.
The per-component ablation series was not completed, so the conclusion applies only to
the complete fixture configuration. See `docs/EXPERIMENTS.md` for the complete result
table and evidence boundaries.

The checksums below cover the **complete pretty-printed JSON stdout** generated by the
commands above at `9b5e13a`, including its final newline. They were recomputed on
2026-11-26. The per-step JSON is deliberately not committed; regenerate it in a temporary
artifact directory when reviewing or comparing a report. Checksums from the earlier
post-action dry-run implementation are not interchangeable with these values.

| Report | SHA-256 |
|---|---|
| discovery `cyclic_shift` | `e4090aad9dc018fb2deca7116230070552012bea721cc376e35675a3ce4f49c1` |
| discovery `nonstationary` | `641ad1759f20f07661d6f63db58c0c97e7ffc5e26179769c6110664140c924bc` |
| discovery `regime_shift` | `670e7905da3d92a842af3b8eb336239afd4aa2cc6a4a6c5de5631960805c95bd` |
| discovery `stable` | `a42800af9cf86fb424a16be3c42530697e5c84b4c25b872d7dc36059cfc9fc81` |
| validation `cyclic_shift` | `cba17e5f7fa6be78e451447868e4dc1d94be38274fbd3a6e3bea04b8dcf9e78c` |
| validation `nonstationary` | `a12a2cf4e779c8465ca428cf76785aac1b9103f44038f270341363be49041f58` |
| validation `regime_shift` | `4260f68fdd6cadc420d8f924b81416fbb7acee22bed209f03ae33075ae15f2a8` |
| validation `stable` | `a6b7b7a06d4130721f259c44a1228156452ceb26d6b5cf934746fcd5dab03b42` |

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
- `pipeline` (`--diagnostics`): `proposals` counts new commits produced at the current step. `dry_runs` clone the UCB snapshot **before** the current action, clear its already-consumed policy, then compare KEEP_L0 with the current pending policy. `candidate_changes`, `selected_changes`, and `candidate_changes_not_selected` distinguish modulation from actual execution and L0 dwell/margin/safety holds. `selected_changes` is a counterfactual arm difference, not necessarily an attributed L2 intervention: an invalid, cleared, or no-op policy can still alter a dry-run without satisfying `l2_policy_applied`. Per-step `applied` belongs to the action actually taken; it is not the proposal queued for the next step.
- `mean_committed_policies_applied`: committed snapshots whose recorded decision changed the original UCB candidate and selected that candidate, as distinct from proposals, queued commits, no-op policies, and safety-blocked execution;
- `mean_expert_committed` / `mean_expert_applied` / `mean_expert_not_applied`: per-expert commit and actual-action-attribution counts. Only an applied commit receives the action's external reward in `mean_expert_reward`; a rejected or no-op proposal is not credited to its proposed expert. KEEP_L0 retains its actual selection propensity for feedback without becoming an intervention;
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

State schema v6 stores the resident controller, online Dayan state, bounded cache, validated pending/applied policy snapshots, UCB statistics, decision explanations, and a per-instance case-replay cursor. Missing legacy L2 permissions default to `false`; malformed or noncommitted snapshots collapse to `KEEP_L0`. Older trajectory lines without the newer fields replay with safe defaults.

## Historical results

The earlier composite constrained-baseline/UCB-treatment results are not evidence for this controller: they changed the decision maker as well as L2. They are intentionally omitted rather than presented as current acceptance evidence.

## Historical paired result (2026-11-23)

A release-build run used 20 seeds beginning at `20261115`, with 12 episodes × 12 steps, for `nonstationary`, `regime_shift`, and `cyclic_shift`. All three completed successfully.

| Scenario | Reward, off → on | Best-arm rate, off → on | Commits / applied | Cache hits | Warm recovery |
|---|---:|---:|---:|---:|---:|
| `nonstationary` | 0.739931 → 0.740625 | 0.802083 → 0.803125 | 0.00 / 0.00 | 6.95 | `null` |
| `regime_shift` | 0.787153 → 0.787153 | 0.877431 → 0.877431 | 0.00 / 0.00 | 12.85 | `null` |
| `cyclic_shift` | 0.584028 → 0.584028 | 0.526736 → 0.526736 | 0.00 / 0.00 | 10.00 | `null` |

The regime and cyclic runs recovered with the same mean external latency in both branches (29.85 and 6.05 steps respectively). Proposals were vetoed by the Lyapunov gate; there were no executable commits. Cache hits therefore provide **no evidence of warm-start benefit**. The tiny nonstationary reward difference is not accompanied by L0 policy intervention and is not an L2 effectiveness claim. Prediction-error improvement also remains unproven. In the current per-seed report, only seed `20261120` has a nonzero reward delta (`0.01388888888888884`), while all 20 runs have `treatment_influenced_l0 = false`.

Reproduce with the recommended command, replacing `nonstationary` with `regime_shift` and `cyclic_shift` for the other two required runs. Retain the JSON as a CI artifact when publishing trend data. Continue to compare other seed ranges before making broader effectiveness claims or enabling L2 by default.

## Current execution-attribution result (2026-11-25)

After merging `origin/main` at `b642a64`, the upstream L1 spectral reflector, L2 cognitive gate, and telemetry remain wired into the runtime. Their output is added to `spectral_output` and the score breakdown only: it does not set a pending policy, grant an L2 commit, or bypass L0's decision and safety gates. Execution attribution uses the recorded L0 decision rather than treating every valid commit as an executed intervention. A proposed expert receives external reward only when its snapshot actually changed and governed the L0 selection. Otherwise the feedback belongs to KEEP_L0 with the logged selection propensity; the proposal itself remains separate in `l2_proposed_policy`.

A release-build paired run used the CI fixture: `nonstationary`, five seeds `20261115..20261119`, 8 episodes × 8 steps:

```bash
cargo run --locked --release --bin omega-kernel -- benchmark-l2 \
  --scenario nonstationary --runs 5 --episodes 8 --steps 8 \
  --seed 20261115 --json
```

| Mean reward, off → on | Best-arm rate, off → on | Reward delta | Best-arm delta | Commits / actually applied | Cache hits | Error delta, off → on |
|---|---:|---:|---:|---:|---:|---:|
| 0.703125 → 0.703125 | 0.687500 → 0.687500 | 0.000000 | 0.000000 | 2.20 / 0.20 | 6.00 | 0.055699 → 0.058215 |

Only seed `20261118` had an actual L0 intervention; the other four seeds did not. Of 11 total commits, one was actually applied and ten were not. The per-expert means were `[KEEP_L0, STABILIZE, EXPLORE, OBSERVE]`: commits `[0.0, 0.4, 0.2, 1.0]`, actually applied `[0.0, 0.2, 0.0, 0.0]`, and not applied `[0.0, 0.2, 0.2, 1.0]`. Thus the nonzero commit count is not evidence that all proposals executed.

**The mechanism is observable, but no benefit is demonstrated:** both reward and best-arm-rate deltas are zero, and mean error delta is slightly worse with L2. Five seeds are insufficient for an effectiveness or production-default claim. No gate parameters or metrics were tuned to improve this result. Any policy tuning must be a separate experiment with a new paired measurement.

Historical verification on the 2026-11-25 source snapshot: 150 default Rust tests and 151 all-feature Rust tests passed with `cargo nextest`; default/all-target and all-feature/all-target Clippy passed with warnings denied; formatting passed. Python 3.13.18 passed all 11 installed-extension tests using a local `maturin --compatibility off` wheel. That local wheel was **not** manylinux2014 certification: the host build's glibc symbols were too new, so the manylinux2014 compliance check failed and remained for the containerized CI workflow. These historical counts are not the current tree's test totals.

## CI / regression use

The JSON output supports CI trend tracking and plotting. Keep this benchmark observational rather than enforcing a hard reward threshold until matched-policy L2 ablations and repeated scenarios provide stable evidence. After collecting stable results across several commits, add a regression threshold with an explicit tolerance and record the benchmark parameters in the report.
