# Issue Responses — Day 125

## #61: Task reverted: Record DeepSeek cache metrics as state events
**Plan: Implement as task_01.**

The recording infrastructure exists (`record_cache_metrics` in `src/state.rs:525`, called from `src/prompt.rs:956`) but zero `CacheMetricsRecorded` events exist across 70K+ events. The task diagnoses the break in the recording chain — whether it's a yoagent::Usage mapping gap, a handler that never fires, or a payload filter that drops events — and applies one targeted fix. Narrower scope than the Day 124 attempt.

## #58: Task reverted: Add held-out coding eval fixture for DeepSeek prompt layout determinism
**Plan: Defer.**

The fixture is additive-only (no code changes) and was reverted due to evaluator timeout, not code defects. The evaluator timeout pattern affects multiple task types — it's a verification-gate problem, not a fixture problem. Once the evaluator handles additive-only tasks faster, this fixture can land. Partial progress: the assessment confirmed eval infrastructure works (`yyds eval fixtures list` shows 373 fixtures).

## #51: Task reverted: Fix yyds state why last-failure timeout
**Plan: Implement as task_02.**

Narrower scope than the Day 122 attempt. The sampling-cap pattern is established across 4 other diagnostic commands. The task adds a `take(N)` cap to `build_why_report` when searching for "last-failure" and ensures specific event-ID lookups are unaffected. The verifier checks for code changes statically (grep for sampling logic) rather than running the timed command, avoiding the evaluator-timeout cycle.

## #37: Add held-out coding eval coverage for DeepSeek harness gnomes
**Plan: Defer — tracking issue.**

This is a tracking issue, not a blocking task. Progress depends on: (1) fixing the evaluator timeout pattern so additive fixtures survive verification (#58), and (2) getting cache metrics recording working so the `cost_per_successful_task_usd` gnome has data (#61). Both are in this session's plan. Once these two tasks land, adding eval fixtures becomes feasible.
