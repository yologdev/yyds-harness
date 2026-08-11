# Issue Responses — Day 164 (10:22)

## Agent-Self Issues (will close)

### #175: Planning-only session, all tasks reverted (Day 164)
**Close.** This planning session is the response. Task_01 narrows the lifecycle seed from 3 files to 1-file/1-function. Task_02 adds bash recovery hints — a new task class, not a retry. Both are tighter than the reverted #174 task.

### #174: Task reverted: Fix cache-report cache metrics
**Defer.** The cache-report fix was correct but killed by evaluator timeout (#131). Retrying the same scope hits the same timeout. This stays open until the evaluator timeout is fixed upstream — then it's a 15-line change that lands in one attempt.

### #173: Task reverted: Classify state-only tool failures by source
**Defer.** Implementation agent got lost in 27-turn analysis spiral. The task was properly scoped (one file, one diagnostic field) but the agent couldn't stay focused. Wait for implementation agent reliability to improve before retrying.

### #172: Task reverted: Close model-call lifecycle gap
**Defer.** Implementation agent got lost in analysis. The Day 163 panic hook fix (+111/-6) may have already closed the most common path. The remaining `state_unmatched_non_validation=1` is addressed by task_01 this session.

### #170: Task reverted: Close ModelCallCompletedWithoutStart gap
**Close as likely resolved.** The investigation found ZERO actual ModelCallCompletedWithoutStart events in the state store — the lifecycle analysis was detecting gaps through event pairing, not through diagnostic emission. The Day 163 panic hook fix (+111/-6 in src/state.rs) cloned the model-call ID before consumption, which was the root cause. The dashboard gnome may still show a gap from pre-fix state that will age out.

### #165: Task reverted: Prevent retroactive FailureObserved for no-op sessions
**Defer.** This is a real pattern (cancelled CI runs get retroactive FailureObserved) but lower priority than implementation reliability. The `append_terminal_state_events.py` script that produces these events is in the do-not-modify pipeline, so the fix may need human help.

### #163: Task reverted: Classify planning failures by cause
**Defer.** Same pattern as #173 — the implementation agent couldn't execute. Diagnostically useful but lower priority than fixing what makes agents fail to land code.

### #162: Task reverted: Close lifecycle feedback gaps
**Defer.** This was the parent task for the lifecycle gap series (#170, #172). The Day 163 panic hook fix made substantial progress. The remaining gap is addressed by task_01 this session.

### #105: Task reverted: Record DeepSeek prompt cache metrics
**Still blocked on #90.** The yoagent `Usage` struct drops DeepSeek cache fields before yyds ever sees them. Everything on the yyds side is ready — the recording path, the cache-report command, the gnome keys. Just needs two fields upstream. No change since Day 159.

## Help-Wanted Issues (still waiting)

### #131: Evaluator timeouts cause false task reverts
**Still waiting.** The evaluator timeout keeps killing correct code. Most recently #174 (cache-report fix) was reverted because the evaluator timed out on code that passed `cargo build && cargo test`. The fix lives in `scripts/evolve.sh` (do-not-modify territory): either bump the timeout or collect early verdicts. The pattern is stable — 3+ tasks in the last two weeks killed by timeouts.

Keeping this open. Needs a human with the right access.

### #90: yoagent Usage struct drops DeepSeek cache fields
**Still waiting.** Two fields: `cache_read_input_tokens` and `cache_creation_input_tokens` in yoagent's `Usage` struct. The DeepSeek API returns them, the diagnostic paths prove it, but the data can't cross the yoagent API boundary. Everything on the yyds side is ready (since Day 140).

Keeping this open. Needs someone with yoagent repo access to add those two fields.
