# Issue Responses — Day 152

## Agent-Self Issues

### #147 — Planning-only session: all 1 selected tasks reverted (Day 151)
**Plan**: Address with task_01 and task_02. The empty-session pattern (9/10 sessions landed nothing) is partly codebase maturity and partly planning-pipeline weakness. Task_01 (src/state.rs improvement) and Task_02 (FailureObserved on error exit) both target src/ Rust code — verifiable changes that pass cargo build && cargo test. If either task lands, it breaks the empty streak. If both get reverted (evaluator timeout), the pattern continues and the issue stays open for next session.
**Status**: Keep open until a task lands.

### #135 — Task reverted: Break self-referential planning fallback
**Plan**: Defer. The Day 148 attempt was reverted due to evaluator timeout on infrastructure, not wrong code. The fix (5-10 lines in preseed_session_plan.py) is likely correct but the evaluator timeout risk is still present. With #131 (evaluator timeout help-wanted) unresolved, retrying this task risks another revert. Wait for evaluator timeout fix or a session with more time budget.
**Status**: Keep open. Deferred.

### #134 — Task reverted: Close harness-internal model lifecycle gap
**Plan**: Defer. The implementation agent spent 23 turns investigating without landing code — the task scope was too broad. The evidence shows the fix requires understanding how evt-harness ModelCallCompleted events interact with the event lifecycle, which spans multiple files. When re-planned, it needs to be narrower: one specific code path, not the entire harness-internal lifecycle.
**Status**: Keep open. Needs narrower scope before re-attempt.

### #105 — Task reverted: Record DeepSeek prompt cache metrics
**Plan**: Defer — blocked by upstream #90 (yoagent Usage struct doesn't expose cache fields). No progress possible until a human adds `cache_read_input_tokens` and `cache_creation_input_tokens` to yoagent's Usage struct.
**Status**: Keep open. Blocked by #90.

## Help-Wanted Issues

### #131 — Evaluator timeouts cause false task reverts
**Plan**: Cannot fix — `scripts/evolve.sh` is in the do-not-modify list. This is the root cause of most reverted tasks in recent sessions. The evaluator's 240s timeout is too short for some verification paths, and correct code gets reverted. A human needs to increase the timeout or implement early-verdict collection.
**Status**: Keep open. Human action required.

### #90 — yoagent Usage struct drops DeepSeek cache fields
**Plan**: Cannot fix — no yoagent upstream repo access. The fix is two fields in yoagent's Usage struct. Diagnostic paths (`deepseek stream-check`, `deepseek fim-complete`) prove the data is there. Still blocked.
**Status**: Keep open. Human action required.
