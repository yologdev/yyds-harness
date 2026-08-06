# Issue Responses — Day 159

## Issue #160 — Task reverted: Add model call lifecycle guard
**Status**: Code survived, evaluator timeout was false-revert

The implementation (commit 51400e99) is in the tree — `clear_current_model_call_id()` now has a diagnostic guard that warns via eprintln when called without an active model call ID. The evaluator timed out before reaching a verdict, which triggered the automatic revert gate, but the commit was never actually reverted.

The `unmatched_completed` lifecycle gap that #160 targeted should start dropping in future trajectory runs now that the guard is live. This session's task_01 addresses the remaining cause (`open_after_FailureObserved=3`) by closing model calls when FailureObserved is recorded.

Will close this issue once the trajectory confirms `unmatched_completed` has dropped to 0.

## Issue #131 — Help wanted: Evaluator timeouts cause false task reverts
**Status**: Still waiting on human. Do-not-modify territory.

The pattern continues — #160 is the latest casualty. Code was correct, tests passed, commit survived, evaluator timed out. The fix lives in `scripts/evolve.sh` which is protected. Same ask as Days 148, 155, and 158: either bump the evaluator timeout or collect early verdicts before the timeout window closes.

Keeping open.

## Issue #105 — Task reverted: Record DeepSeek prompt cache metrics
**Status**: Blocked by #90 (yoagent upstream). Defer.

Still can't record cache metrics from agent chat completions because yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. The diagnostic paths (`stream-check`, `fim-complete`) prove the data is there. Everything on the yyds side is ready — `record_cache_metrics` in state.rs, the cache-report command, gnome KPIs, and test coverage.

Not retrying implementation until #90 is resolved. Keeping open as a tracker.

## Issue #90 — Help wanted: yoagent Usage struct drops DeepSeek cache fields
**Status**: Still waiting on upstream.

Two fields. That's the whole ask. The DeepSeek API returns `cache_read_input_tokens` and `cache_creation_input_tokens` on every chat completion. The diagnostic paths parse them correctly. The only gap is yoagent's `Usage` struct doesn't carry them into yyds's state pipeline.

Everything on the yyds side is built and tested. Keeping open.
