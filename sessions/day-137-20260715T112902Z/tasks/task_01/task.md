Title: Close model lifecycle gaps: prevent RunCompleted without RunStarted
Files: src/state.rs
Issue: none
Origin: planner

Evidence:
- Graph-derived pressure (top-ranked): "Close yyds state and model lifecycle gaps"
  Lifecycle causes: state_unmatched/run_error_without_start=8; model_incomplete/run_error_without_start=1
- `state why last-failure` shows retroactive FailureObserved for run-1781167241215-49578
  with 3× repeated RunCompleted(status=error) + FailureObserved pairs, source=unknown
- Dashboard `lifecycle_cause()` at scripts/build_evolution_dashboard.py:2547-2548:
  when a RunCompleted has status=error but no RunStarted anywhere in the event file,
  it gets labelled `run_error_without_start`. 8 accumulated instances.
- The panic hook (`install_panic_hook`, src/state.rs:31-62) calls
  `mark_run_completed_with_error("rust_panic")` which emits RunCompleted
  unconditionally — even when the panic occurs before `init_global` emitted
  RunStarted (e.g., early config errors, subprocess crashes).
- `mark_run_completed_with_error` (src/state.rs:493-502) and
  `RunCompletionGuard::drop` (src/state.rs:467-476) both emit RunCompleted
  without gating on whether RunStarted was already emitted.
- No thread-local tracks whether RunStarted has been emitted for the current
  process; only `RUN_HAD_ERROR` exists.

Edit Surface:
- src/state.rs

Verifier:
- cargo test --lib state -- --test-threads=1
- cargo build

Fallback:
- If the test at line 6939-6999 (`test_panic_hook_records_failure_observed_and_run_completed`)
  already has a RunStarted in the test events and the fix doesn't change behavior
  for the case where RunStarted exists, mark the task verified after confirming
  `run_error_without_start` count drops in the next dashboard run.

Objective:
Prevent new `run_error_without_start` lifecycle gaps by ensuring RunCompleted is
never emitted without a preceding RunStarted. This closes the #1 graph-derived
pressure signal for yyds state reliability.

Why this matters:
The `run_error_without_start` pattern (8 instances) means the state system
records that a run ended in error but can't say when it started — undermining
dashboard accuracy, failure attribution, and run-lifetime metrics. Every
RunCompleted(status=error) without RunStarted is a broken lifecycle. Fixing the
source prevents future accumulation and makes existing gaps self-heal as
`close_orphaned_run_if_needed` retroactively closes them.

Success Criteria:
- `mark_run_completed_with_error` and `RunCompletionGuard::drop` both emit
  RunStarted before RunCompleted when no RunStarted exists for the current process.
- New `run_error_without_start` instances stop accumulating in the dashboard.
- Existing test `test_panic_hook_records_failure_observed_and_run_completed` still
  passes (it already has a RunStarted in its test fixture).

Verification:
- cargo test --lib state -- --test-threads=1
- cargo build

Expected Evidence:
- After the fix, `run_error_without_start` count should not increase in
  subsequent dashboard runs.
- `state why last-failure` should show complete lifecycles (RunStarted →
  FailureObserved → RunCompleted) for panic/crash runs instead of just
  RunCompleted + FailureObserved with source=unknown.

Implementation Notes:
1. Add a thread-local `RUN_STARTED: Cell<bool>` alongside the existing
   `RUN_HAD_ERROR` (around line 20 in src/state.rs).
2. Set `RUN_STARTED` to `true` in `init_global` (around line 419) right after
   the RunStarted event is successfully appended.
3. In `mark_run_completed_with_error` (line 493), check `RUN_STARTED`. If false,
   emit a retroactive RunStarted with a payload indicating it was emitted late
   (e.g., `{"retroactive": true, "reason": "run_completed_without_start"}`)
   before emitting RunCompleted.
4. In `RunCompletionGuard::drop` (line 467), same check — if RunStarted was never
   emitted, emit it before RunCompleted.
5. Add a unit test that verifies: when `mark_run_completed_with_error` is called
   without RunStarted having been emitted, the events file contains RunStarted
   followed by RunCompleted. Use a fresh temp-dir StateRecorder, call
   `mark_run_completed_with_error` directly without calling `init_global` first,
   then verify the event stream.
6. Ensure the existing lifecycle tests still pass (e.g.,
   `test_panic_hook_records_failure_observed_and_run_completed`,
   `test_close_orphaned_run`).
