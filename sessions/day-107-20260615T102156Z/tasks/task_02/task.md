Title: Close model lifecycle gap — model_completion_without_start
Files: src/deepseek.rs
Issue: none
Origin: planner

Objective:
Emit a ModelCallStarted event before every model API call so the state lifecycle never records model_completion_without_start. This closes the highest-ranked trajectory pressure item.

Why this matters:
YOUR TRAJECTORY graph pressure #1: "Close yyds state and model lifecycle gaps (deepseek_model_call_abnormal_completed_count=1): Lifecycle causes: model_abnormal/model_completion_without_start=1"

The harness records ModelCallCompleted events but occasionally misses the corresponding ModelCallStarted event, creating a lifecycle gap where a completion exists without a matching start. This makes model call lifecycle tracking unreliable and contributes to non-proven claims in the structured state snapshot (model_lifecycle=2 missing claims).

The gap likely occurs when model calls go through a code path that emits completion events but doesn't emit the start event — an asymmetry in the event-recording wrapper.

Success Criteria:
- All model call completions have a matching start event in the state lifecycle
- `state lifecycle` no longer shows model_completion_without_start
- No regression in model call functionality or streaming
- `cargo test` passes with DeepSeek-related tests

Verification:
- cargo build && cargo test deepseek
- ./target/debug/yyds deepseek doctor (config still works)
- ./target/debug/yyds state lifecycle (model lifecycle clean, no abnormal completions over time)
- Run a quick prompt to generate fresh model call events, then check state lifecycle

Expected Evidence:
- model_completion_without_start count drops to 0 in subsequent sessions
- model_lifecycle non-proven claims decrease in structured state snapshot
- ModelCallStarted events appear in `state tail` output for every model call

Implementation Notes:
- Look at where ModelCallStarted and ModelCallCompleted events are emitted in `deepseek.rs`. Find code paths that emit completion but skip the start event.
- Common cause patterns:
  - Retry/fallback paths that enter mid-call without the start event
  - Error-handling branches that emit a completion status without having emitted start
  - Streaming callbacks that fire completion but started from a different entry point
- The fix should ensure every entry point that can produce a completion also emits a start.
- If the gap is in the prompt execution layer (e.g., `prompt.rs` or `prompt_retry.rs`) rather than `deepseek.rs`, note the actual file and create a narrow follow-up rather than expanding scope.
- Keep the change minimal — a one-line event emission at the right call site is better than restructuring.
