# Harness Evolution Goal

The goal of this project is to make **yyds** an excellent DeepSeek-native
coding and general-purpose agent. The evolution harness exists to improve the
agent, not to make the dashboard look healthy.

Every evolution cycle should answer one question:

> Did this change measurably improve yyds as a DeepSeek agent, or did it only
> improve our ability to observe the loop?

Both are useful, but they are not the same.

## Fitness Comes First

Fitness gnomes measure agent capability:

- `task_success_rate`
- `task_verification_rate`
- `task_mechanical_verification_rate`
- `coding_log_score`
- `session_success_rate`
- `retry_success_rate`
- `json_parse_failure_rate`
- `tool_call_malformed_rate`
- `context_miss_rate`
- `repair_loop_count`
- `cost_per_successful_task_usd`
- `latency_per_successful_task_ms`

These are the metrics that should decide whether yyds is becoming better at
coding, tool use, following instructions, recovering from failures, and using
DeepSeek efficiently.

## Diagnostics Are Gates

Diagnostic gnomes explain whether the feedback loop can be trusted:

- `planner_no_task_count`
- `provider_error_count`
- `evaluator_timeout_count`
- `task_artifact_coverage`
- `task_lineage_capture_coverage`
- `state_capture_coverage`
- `state_operational_capture_coverage`
- `audit_capture_coverage`
- `protected_file_revert_count`
- `task_revert_count`

These metrics are gates. Fix them when they block accurate fitness measurement
or cause wasted evolution turns. Do not optimize them as the final goal.

## Task Selection Rule

Future task selection should prefer work that can name all three:

1. The fitness gnome it should improve.
2. The focused verifier or held-out eval that proves the improvement.
3. The expected artifact in task lineage, state events, `fitness.json`, or the
   evolution dashboard.

If a task only improves a diagnostic gnome, it must say which fitness signal was
blocked by that diagnostic gap.

## Promotion Rule

A landed patch counts as meaningful evolution only when at least one of these is
true:

- It raises a fitness gnome.
- It prevents a recurring failure that was suppressing fitness measurement.
- It adds a stable held-out eval that will measure future fitness.
- It removes stale task pressure that was causing wasted DeepSeek attempts.

Workflow success alone is not enough.

## Immediate Direction

The next harness phase is:

1. Keep the loop honest: no fake task success, no empty-selection success.
2. Make capability fitness visible in every trajectory.
3. Prefer fitness-backed task plans over dashboard-only cleanup.
4. Preserve per-session `fitness.json` as a durable artifact.
5. Retire stale recurring task pressure before it burns another DeepSeek turn.

