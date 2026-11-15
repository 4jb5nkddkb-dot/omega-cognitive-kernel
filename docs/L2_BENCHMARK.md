# L2 Benchmark

## Purpose

`benchmark-l2` compares the same seeded non-stationary two-armed bandit task with L2 interventions disabled and enabled. It is intended to turn L2 from a qualitative heuristic into a measurable, repeatable capability.

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

The two arms use identical seeds:

- **baseline**: L2 monitor may be evaluated, but L2 cannot modify biases, constraints, or `RuleEffect`;
- **L2 enabled**: normal L2 detector, fixed-point, modulation, and effect-nudge behavior.

The short benchmark fixture lowers the treatment `commit_threshold` to `0.70` so the candidate-to-commit path is exercised. The production/default L2 threshold remains `0.82`; this fixture is for mechanism measurement, not a production configuration recommendation.

Available scenarios:

- `stable`: optimal arm remains stable; tests over-intervention;
- `nonstationary`: optimal arm switches; tests adaptation;
- `degraded`: the environment and prediction signal degrade halfway through; tests consequence feedback and exit behavior.

Each run uses a fresh temporary state path. Persistent benchmark artifacts are removed after the run.

## Metrics

- `mean_reward`: mean reward per bandit decision;
- `best_arm_rate`: fraction of decisions choosing the currently optimal arm;
- `mean_error_delta`: final prediction error minus initial prediction error; lower is better;
- `error_decrease_rate`: fraction of seeded runs where final error is lower than the initial error;
- `mean_patches_accepted`: average accepted L1 patches per run;
- `mean_l2_nudges`: average steps where L2 produced an effect nudge;
- `mean_l2_consequence_score`: short-term reward/error consequence score;
- `mean_fixed_point_enters` / `mean_fixed_point_exits`: L2 regime transition counts;
- `mean_final_alignment_score`: final L2 alignment score.

The top-level `delta_*` fields are:

```text
L2 enabled aggregate - baseline aggregate
```

For reward and best-arm rate, positive is better. For error delta, lower is better; the JSON report keeps each arm's value so that interpretation is not hidden.

## Interpretation

A useful L2 result should satisfy all of the following over repeated seeds:

1. reward or best-arm rate improves against baseline;
2. the improvement is not caused only by a higher patch count;
3. error delta does not regress materially;
4. L2 nudges occur in the intended operating context, and fixed-point entries are followed by exits when conditions degrade;
5. results remain directionally stable when the seed range changes.

Do not use `mean_l2_nudges > 0` or `mean_fixed_point_enters > 0` as a success criterion by itself. Those only prove that an L2 path fired, not that the intervention helped.

## CI / regression use

The JSON output is designed for a later CI job or plotting script. Initially, keep this benchmark observational rather than enforcing a hard reward threshold. After collecting stable results across several commits, add a regression threshold with an explicit tolerance and record the benchmark parameters in the report.
