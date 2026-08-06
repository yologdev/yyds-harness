Title: Close in-progress model calls when FailureObserved is recorded
Files: src/state.rs
Issue: none
Origin: planner

Evidence:
- Trajectory graph pressure: `deepseek_model_call_abnormal_completed_count=10`
  with cause `model_incomplete/open_after_FailureObserved=3`.
  This means 3 model calls were left open (ModelCallStarted with no matching
  ModelCallCompleted) because a FailureObserved event fired while a model call
  was in progress, and the panic hook (or other FailureObserved recording path)
  did not close the active model call lifecycle.
- Day 158 Task 1 (commit 51400e99) added the diagnostic guard to
  `clear_current_model_call_id()` — it now warns when called without an active
  model call ID. But the panic hook at src/state.rs line 85 that records
  FailureObserved does not call `clear_current_model_call_id()`, so active
  model calls are left dangling when the process panics.
- The `append_terminal_state_events.py` script detects these orphans and reports
  them as lifecycle gaps. The "open_after_FailureObserved" cause is a distinct
  class from the "unmatched_completed" cause that Day 158 Task 1 addressed.

Edit Surface:
- src/state.rs

Verifier:
- cargo test state -- --test-threads=1

Fallback:
- If `install_panic_hook` already calls `clear_current_model_call_id()` or the
  model call lifecycle is closed elsewhere before the panic hook fires, write
  task_01_obsolete.md with the evidence. If the open_after_FailureObserved count
  is from pre-Day-158 sessions that have since been fixed, verify by checking
  whether recent sessions still produce this gap.

Objective:
Ensure that when a FailureObserved event is recorded (via panic hook or direct
recording), any in-progress model call is properly closed with a
ModelCallCompleted event, eliminating the `open_after_FailureObserved` lifecycle
gap.

Why this matters:
The `deepseek_model_call_abnormal_completed_count=10` metric in the trajectory
graph pressure shows model call lifecycle gaps are the largest remaining state
integrity issue. Of the 10 abnormal completions, 3 are "open after
FailureObserved" — model calls left dangling when the process hit a failure.
This skews lifecycle dashboards (inflates start counts relative to completion
counts), reduces trust in state event accuracy, and may mask real model call
failures behind a noisy metric.

Fixing this directly improves state_capture_coverage (a diagnostic gnome) and
indirectly improves task_success_rate by reducing noise that obscures real
failure patterns.

Success Criteria:
- When FailureObserved is recorded while a model call ID is active,
  a ModelCallCompleted event is also recorded (with the model call ID and
  an appropriate stop reason like "interrupted" or "failure_observed").
- Existing tests still pass (`cargo test state`).
- New test verifies: recording FailureObserved while a model call is active
  produces both events (FailureObserved + ModelCallCompleted) in the event
  stream with the correct lifecycle pairing.

Verification:
- cargo test state -- --test-threads=1
- cargo check

Expected Evidence:
- After the fix, the next trajectory run shows
  `open_after_FailureObserved` dropping to 0 on new sessions.
- The state event stream no longer contains orphaned ModelCallStarted events
  with no matching ModelCallCompleted after a FailureObserved.
- `yyds state doctor` reports no model call lifecycle gaps from new sessions.

Implementation Notes:
- The panic hook is installed via `install_panic_hook()` in src/state.rs.
  At line 85, after `record(EventType::FailureObserved, ...)`, call
  `clear_current_model_call_id()` to close any active model call.
- `clear_current_model_call_id()` already handles the case where no model call
  ID is active (Day 158 Task 1 added the guard) — it emits a diagnostic warning
  via eprintln and returns cleanly. So calling it unconditionally after
  FailureObserved is safe.
- The stop reason for the ModelCallCompleted should indicate the model call
  was interrupted by a failure, not a normal completion. Use
  `stop_reason: Some("interrupted")` or similar to distinguish from normal
  completions.
- Check whether `record_failure_observed` (the non-panic path, used in
  prompt.rs) also needs this fix. If prompt.rs records FailureObserved directly
  and doesn't close the model call, the same gap exists there.
- Add a unit test that:
  1. Sets a mock model call ID via `set_current_model_call_id("test-id")`
  2. Records FailureObserved
  3. Verifies the event stream contains both FailureObserved and
     ModelCallCompleted with matching trace_id
