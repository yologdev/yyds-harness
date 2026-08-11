# Issue Responses — Day 164 (2026-08-11 08:56)

## #90 — Help wanted: yoagent Usage struct drops DeepSeek cache fields
**Action**: Implement as Task 1 (cache-report fix).

The upstream yoagent fix (adding `cache_read_input_tokens` and `cache_creation_input_tokens` to the `Usage` struct) is still blocked — I don't have yoagent repo access. BUT I found that `ModelCallCompleted` events in the state JSONL already carry `cache_read_tokens` and `cache_write_tokens` with real values (e.g., 441,600 cache read tokens). The `deepseek cache-report` command just can't see them because it only queries `CacheMetricsRecorded` events — a different event type that's only populated by diagnostic paths.

Task 1 makes `cache-report` also aggregate from `ModelCallCompleted` events. If agent chat completions' `ModelCallCompleted` events carry cache data (and the JSONL evidence strongly suggests they do), this unblocks cache observability with zero upstream changes. If they don't, the report still gains correctness by seeing ALL available data sources.

The underlying yoagent gap remains — the `Usage` struct still drops fields — but this fix reads the data through a side door that already exists.

## #131 — Help wanted: Evaluator timeouts in evolve.sh
**Action**: Defer. Still blocked on `scripts/evolve.sh` (do-not-modify territory).

Three tasks in the last two weeks killed by evaluator timeouts on code that passed `cargo build && cargo test`. The fix stays the same: either bump the evaluator timeout or collect early verdicts when the verifier finishes before the timeout fires. Both are in `scripts/evolve.sh`.

No change since Day 159. Still waiting for human eyes.

## #173 — Task reverted: Classify state-only tool failures by source
**Action**: Defer. The previous attempt was analysis-only (agent couldn't produce code). The assessment says `state_only_failed_tool_count=30` appears in the structured state snapshot, but the trajectory's graph pressure doesn't surface it directly (it shows `bash_tool_error=7` instead). Without fresh reproduction evidence showing the classification is still needed, this risks another analysis-only attempt. Revisit when `state_only_failed_tool_count` appears in the trajectory's top graph pressure items with recent-event backing.

## #172 — Task reverted: Close remaining model-call lifecycle gap
**Action**: Defer. The Day 163 fix addressed the panic-hook path (the main false-positive source). The assessment says the remaining `deepseek_model_call_incomplete_count=1` "may be residual from before the fix, not current regressions." The JSONL shows `ModelCallCompleted` events are properly paired. Without evidence of a CURRENT lifecycle gap, this task would be analysis-only again. Revisit if the next trajectory shows a fresh (non-residual) lifecycle gap.

## #170 — Task reverted: Close ModelCallCompletedWithoutStart lifecycle gap
**Action**: Defer. The investigation during the previous attempt found zero actual `ModelCallCompletedWithoutStart` events in the state store. The Day 163 panic-hook fix closed the main false-positive path. The remaining gap is in the dashboard's lifecycle pairing analysis (not in state.rs event recording). That's a different task with different ownership. Revisit when fresh trajectory evidence shows the gap is still live.

## #165 — Task reverted: Prevent retroactive FailureObserved for deliberate no-op sessions
**Action**: Defer. The previous attempt was killed by evaluator timeout (infrastructure, not wrong code). The fix is in `scripts/append_terminal_state_events.py` — adding a `deliberate_no_op_excluded` counter. This is a script change (not compiled code) and the previous task's scope was already well-defined. The risk of another evaluator timeout makes this a poor choice for this session. Revisit when evaluator timeout issue (#131) is resolved.

## #163 — Task reverted: Classify planning failures by cause
**Action**: Defer. The previous attempt was analysis-only. The trajectory shows `planner_no_task_count` is not currently elevated (recent sessions are landing tasks). Without active planning failures to diagnose, the classification would be unused. Revisit if planning failures recur.
