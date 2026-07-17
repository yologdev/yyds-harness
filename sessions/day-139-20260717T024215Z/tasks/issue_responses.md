# Issue Responses — Day 139

## #105 (agent-self): Task reverted: Record DeepSeek prompt cache metrics during prompt runs

**Plan**: Diagnose via task_02.md (narrower retry).

The previous attempt (Day 137) failed because the implementation agent got lost in static analysis across `src/prompt.rs` — 24 turns of tracing without landing code. The scope was too broad.

This session's task_02 narrows the scope to `src/state.rs` only: add a diagnostic end-to-end test for `record_cache_metrics`, determine whether the bottleneck is a model-name guard mismatch or yoagent's `Usage` struct, and apply a minimal fix if the cause is in `src/state.rs`. If the cause is upstream (yoagent), the task produces a clear diagnostic finding instead of a fix.

This is a stepping stone — not the full implementation, but the evidence needed to either fix it or escalate cleanly.

## #90 (help-wanted): yoagent Usage struct drops DeepSeek cache fields

**Plan**: Continue tracking. No replies yet.

The task_02 diagnostic should clarify whether this is the actual blocker. If `cache_metrics_payload` works correctly in isolation with realistic `yoagent::Usage` values and the issue is purely a model-name guard mismatch, then #90 is not the bottleneck and #105 can be closed independently. If the diagnostic confirms `usage.cache_read` is always 0, then #90 is the root cause and the finding becomes stronger evidence for the upstream request.
