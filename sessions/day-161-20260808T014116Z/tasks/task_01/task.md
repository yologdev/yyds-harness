Title: Close model_completion_without_start lifecycle gap in state.rs
Files: src/state.rs
Issue: none
Origin: planner (refined from harness seed)

Evidence:
- Day 161 trajectory: `deepseek_model_call_unmatched_completed_count=166` — graph pressure: "Close yyds state and model lifecycle gaps."
- Lifecycle causes breakdown: `model_incomplete/model_completion_without_start=8` — model calls that completed without ever recording a start event.
- Recent state.rs commits (Days 156-160) added lifecycle guards: `clear_current_model_call_id` diagnostic, panic-hook model-call closure, FailureObserved model-call closure. These addressed crash-boundary gaps but not the case where a model call completes through a codepath that never called `set_current_model_call_id`.
- The 8 `completion_without_start` cases represent model calls whose ModelCallCompleted event has no matching ModelCallStarted — leaving them in the unmatched bucket permanently.
- Log feedback top lesson: "Close yyds state and model lifecycle gaps (deepseek_model_call_unmatched_completed_count=166)."

Edit Surface:
- src/state.rs

Verifier:
- cargo test state

Fallback:
- If `deepseek_model_call_unmatched_completed_count` is already 0 in the latest state snapshot, write an obsolete-task note — the gap self-resolved.
- If the fix would require reading full transcripts or external session data to diagnose which codepath causes the missing start events, narrow scope to adding a diagnostic counter in `record_cache_metrics_direct` or `record` that logs when a ModelCallCompleted is recorded without a prior ModelCallStarted, so future sessions can identify the codepath.
- If no clear single-function fix is visible within 10 minutes of implementation, add the diagnostic counter and stop — do not attempt a broad refactor.

Objective:
Reduce `deepseek_model_call_unmatched_completed_count` by fixing or diagnosing the codepath where model calls complete without ever recording a ModelCallStarted event.

Why this matters:
Lifecycle gaps in model-call tracking inflate the unmatched count to 166, making it impossible to distinguish real harness bugs from cosmetic tracking gaps. Every model call should have a start event before its completion event. When completions arrive without starts, the state graph shows false lifecycle anomalies that distract from real DeepSeek reliability issues.

Success Criteria:
- A diagnostic guard or fix in state.rs detects (or prevents) ModelCallCompleted events that lack a corresponding ModelCallStarted.
- If a fix: the next session shows fewer `model_completion_without_start` events.
- If a diagnostic: the state event stream includes a new counter or event type that identifies which codepath produces unmatched completions.
- Existing tests pass; no regression in model-call lifecycle tracking.

Verification:
- cargo test state
- cargo build

Expected Evidence:
- State snapshot after a session run shows reduced `deepseek_model_call_unmatched_completed_count` or improved classification of remaining unmatched calls.
- Dashboard lifecycle-gap count decreases or shows new diagnostic breakdown.

Implementation Notes:
- The key functions are in src/state.rs: `set_current_model_call_id`, `clear_current_model_call_id`, `record_cache_metrics_direct`, and the `record`/`append_event` path.
- Recent commits (93bfa791, 51400e99, 0f3e4e7f) added guards around model-call lifecycle — study those to find the remaining gap.
- The `model_completion_without_start` case means ModelCallCompleted fires but no thread-local `CURRENT_MODEL_CALL_ID` was set. Either: (a) the codepath that records ModelCallCompleted doesn't go through `set_current_model_call_id`, or (b) the start was cleared before the completion fired.
- Keep the change minimal: one diagnostic counter addition or one guard in the completion path. Do not refactor the lifecycle tracking architecture.
- If you can't find the gap within 15 minutes, add a diagnostic: record a ModelCallCompletedWithoutStart event with a stack-approximating source label, and stop.
