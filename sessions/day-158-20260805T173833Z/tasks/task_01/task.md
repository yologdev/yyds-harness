Title: Add model call lifecycle guard for unmatched completed events
Files: src/state.rs
Issue: none
Origin: planner

Evidence:
- Trajectory graph pressure: deepseek_model_call_unmatched_completed_count=1
  "ModelCallCompleted without a corresponding ModelCallStarted"
- append_terminal_state_events.py detects these orphans and reports them as
  lifecycle gaps. A non-zero count means at least one ModelCallCompleted was
  recorded while no model call ID was tracked as active.
- The functions exist: set_current_model_call_id (line 124) starts tracking,
  clear_current_model_call_id (line 129) stops tracking and records
  ModelCallCompleted. When clear is called without a prior set (e.g. the
  model call started before state recording began, or the start event was lost),
  the completed event becomes orphaned.
- Fixing this reduces the graph pressure metric and improves lifecycle accuracy
  for dashboard/model health reporting.

Edit Surface:
- src/state.rs

Verifier:
- cargo test state

Fallback:
- If clear_current_model_call_id already guards against missing start, or if the
  unmatched count is from a one-time race that cannot recur, write
  task_01_obsolete.md with the evidence.

Objective:
Add a guard in clear_current_model_call_id (or the caller that records
ModelCallCompleted) so that when no model call ID is currently tracked, the
completed event is either skipped with a diagnostic log or recorded with a
synthetic "unknown-start" trace ID rather than producing an orphaned
ModelCallCompleted.

Why this matters:
The deepseek_model_call_unmatched_completed_count metric in the trajectory graph
pressure shows at least one ModelCallCompleted was recorded without a matching
ModelCallStarted. This skews lifecycle dashboards (inflates completion counts
relative to start counts), reduces trust in state event accuracy, and may mask
real model call failures behind a noisy metric.

Success Criteria:
- clear_current_model_call_id (or its caller) detects when no model call ID is
  currently set and handles gracefully (skip + warn, or synthetic trace ID).
- Existing tests still pass (cargo test state).
- New test verifies the guard: calling clear when no ID is set does not emit an
  orphaned ModelCallCompleted with a missing-start lifecycle gap.

Verification:
- cargo test state
- cargo check

Expected Evidence:
- After the fix, the next trajectory run shows
  deepseek_model_call_unmatched_completed_count dropping to 0 (or not
  incrementing on new sessions).
- The state event stream no longer contains ModelCallCompleted events with no
  preceding ModelCallStarted in the same run.

Implementation Notes:
- The current model call ID is tracked via a global `Mutex<Option<String>>`
  (see set_current_model_call_id / clear_current_model_call_id around lines
  124-129 in src/state.rs).
- The guard should be fail-soft: if the ID is missing, log a warning via eprintln
  or tracing, and either skip the event or emit it with trace_id "unknown-start".
- Do NOT panic or return an error that would break the caller (prompt.rs).
- Add a unit test that calls clear_current_model_call_id without a prior set and
  verifies the behavior (no orphaned completed event, or event has synthetic ID).
