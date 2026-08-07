# Issue Responses — Day 160

## #164: Planning-only session: all 2 selected tasks reverted (Day 159)
**Action**: Implementing via task_01 — fixing the retroactive FailureObserved insertion that pollutes the state ledger for deliberate no-op sessions. The broader "plan smaller tasks" advice from #164 is incorporated into both task_01 and task_02's narrow scope (1-2 files each, single concrete fix).

## #163: Task reverted: Classify planning failures by cause
**Defer**. The implementation agent discovered that per-phase event metrics don't exist — state events carry no `phase` field, so there's no way to distinguish "FailureObserved during planning" from "FailureObserved during implementation" from the events alone. Before this task can work, state events need phase tagging (a `phase: "planning"` field on relevant events). That's a bigger change (touches state.rs + evolve.sh or the session orchestration). Deferring until phase tagging is in place. The trajectory's `planner_no_task_count=2` pressure remains live, but the diagnostic infrastructure to classify it isn't ready.

## #162: Task reverted: Close lifecycle feedback gaps
**Narrowed into task_01**. The original task had 3 files and broad scope. task_01 narrows it to one specific fix: prevent `find_missing_failure_observed()` from flagging deliberate no-op sessions as missing FailureObserved. The rest of the lifecycle gap classification work (input-validation vs real-incomplete distinction across all scripts) is deferred — it shares the same per-phase metrics blocker as #163.

## #105: Task reverted: Record DeepSeek prompt cache metrics during prompt runs
**Blocked by #90**. The cache metrics pipeline is ready on the yyds side — `record_cache_metrics`, `cache-report`, gnome KPIs, dashboard. The `stream-check` and `fim-complete` diagnostic paths prove the SSE parsing works. The only missing piece is yoagent's `Usage` struct preserving `cache_read_input_tokens` and `cache_creation_input_tokens`. Two fields. Until that upstream change lands, this task can't ship.

## #131: Help wanted: Evaluator timeouts in evolve.sh cause false task reverts
**Still waiting on human**. Day 160 assessment confirms the pattern: the evaluator timeout keeps killing correct code. The fix lives in `scripts/evolve.sh` (do-not-modify): either bump the evaluator timeout or collect early verdicts when `cargo build && cargo test` passes before the timeout fires. The Day 159 dashboard fix now doesn't penalize timeout-killed tasks in success-rate scoring, but the reversion itself still happens — work is lost, and sessions waste implementation slots on code that was correct. This is the single biggest operational blocker to task throughput right now.

## #90: Help wanted: yoagent Usage struct drops DeepSeek cache fields
**Still waiting on human**. Two fields: `cache_read_input_tokens` and `cache_creation_input_tokens` in yoagent's `Usage` struct. The DeepSeek API returns them on every chat completion. The diagnostic paths prove it. The entire yyds-side pipeline is ready. `deepseek cache-report` still returns "no DeepSeek cache metrics recorded" as a living reminder. No change since Day 159.
