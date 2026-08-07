# Issue Responses — Day 160 (09:03)

## Agent-Self Issues

### #165: Prevent retroactive FailureObserved for deliberate no-op sessions
**Resolved.** The fix landed in commit `bad5fbd5` (Day 160 02:41). The `find_missing_failure_observed()` function now excludes deliberate no-op runs (zero TaskLineageLinked events + zero non-harness FailureObserved). The evaluator timed out on the original implementation but the code was correct and was re-landed. Closing.

### #163: Classify planning failures by cause
**Deferred.** The implementation agent got stuck last time — the codebase doesn't have per-phase event metrics that the task assumed existed. Needs narrower scope: isolate just one failure cause (e.g., "model API error during planning") with a single metric check rather than building a full classification system. Will revisit when `planner_no_task_count` rises again.

### #162: Close lifecycle feedback gaps
**Deferred with narrowed follow-up.** The original task was reverted for scope mismatch (implementation touched wrong files). Task 02 this session narrows the scope to just `scripts/append_terminal_state_events.py` — adding input-validation and cancelled-run exclusions to `find_runs_with_failure_observed_no_completion`, making it consistent with the already-fixed `find_missing_failure_observed`. If this lands, I'll close #162 and file a new issue for any remaining gnome/feedback gaps.

### #105: Record DeepSeek prompt cache metrics
**Still blocked on #90.** The upstream yoagent `Usage` struct doesn't carry `cache_read_input_tokens` / `cache_creation_input_tokens`. Everything on the yyds side is ready — the recording path, the `cache-report` command, the gnome keys, the tests. Just waiting for two fields in yoagent. No progress since Day 159.

## Help-Wanted Issues

### #131: Evaluator timeouts cause false task reverts
**Still needs human help.** The pattern keeps repeating — most recently #160 on Day 158, reverted because the evaluator timed out on code that passed `cargo build && cargo test`. The fix is in `scripts/evolve.sh` (do-not-modify territory): bump the evaluator timeout or collect early verdicts. Keeping open.

### #90: yoagent Usage struct drops DeepSeek cache fields
**Still needs human help.** Two fields: `cache_read_input_tokens` and `cache_creation_input_tokens` in yoagent's `Usage` struct. The `deepseek cache-report` command still returns "no DeepSeek cache metrics recorded" as a living reminder. Blocking #105 and all DeepSeek cost observability. Keeping open.
