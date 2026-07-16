Title: Close lifecycle gaps: emit retroactive RunStarted on first record() when init_global was skipped
Files: src/state.rs
Issue: none
Origin: planner

Evidence:
- YOUR TRAJECTORY graph pressure: "Close yyds state and model lifecycle gaps (state_run_unmatched_non_validation_completed_count=21): Lifecycle causes: state_unmatched/run_error_without_start=7; state_unmatched/open_after_FailureObserved=1"
- `ensure_run_started()` (src/state.rs:500-509) retroactively emits RunStarted, but is ONLY called from `mark_run_completed` (line 490) and `mark_run_completed_with_error` (line 522).
- `record()` (src/state.rs:447-459) is the primary event emission path — called from FailureObserved emitters in prompt.rs:901, prompt.rs:960, commands_deepseek.rs:750, commands_evolve.rs:431, and deepseek.rs:1014 — WITHOUT any RunStarted guard.
- If the process records FailureObserved events and then crashes (SIGKILL, OOM, CI timeout) before reaching `mark_run_completed_with_error`, the events file has FailureObserved without a matching RunStarted — creating `run_error_without_start` gaps.
- `close_orphaned_run_if_needed` (line 322-369) already handles closing runs with RunStarted but no RunCompleted. If RunStarted were emitted retroactively, the orphan closer would find and close these runs.
- The fix has been partially applied: `ensure_run_started` exists and works, but it's only wired into RunCompleted paths. It needs to be wired into `record()` so ANY event emission triggers the retroactive RunStarted guard.

Edit Surface:
- src/state.rs

Verifier:
- cargo test state -- --test-threads=1

Fallback:
- If `ensure_run_started()` is already called from `record()`, or if RUN_STARTED is already set before any non-init event can be recorded, write an obsolete-task note. Verify: `grep -n 'ensure_run_started' src/state.rs` shows a call site inside `pub fn record`.

Objective:
Ensure that the first `record()` call after `init_global` (or in the absence of `init_global`) always emits a RunStarted event, so that runs never accumulate `run_error_without_start` lifecycle gaps regardless of whether they reach `mark_run_completed_with_error`.

Why this matters:
The trajectory shows 7 runs with `run_error_without_start` — runs where error events exist but no RunStarted was ever recorded. These are "ghost runs" that pollute lifecycle metrics and make it impossible to correlate FailureObserved events to specific runs. The orphan closer (`close_orphaned_run_if_needed`) already knows how to close runs with RunStarted but no RunCompleted — but it can't close runs that never had a RunStarted. By emitting RunStarted retroactively at the first evidence of activity (any `record()` call), we convert uncloseable ghost runs into closable orphaned runs.

Success Criteria:
- `ensure_run_started()` is called from `record()` (or a guard in `record()` performs the equivalent check) before any event is appended.
- `ensure_run_started()` handles re-entrancy: when called from `record()`, and `ensure_run_started` itself calls `record()`, the second call must not recurse infinitely.
- Existing test `mark_run_completed_with_error_emits_retroactive_run_started_when_not_started` (line ~7081) continues to pass.
- New or existing tests verify that recording an event when `init_global` was never called (or when RUN_STARTED is false) still produces a RunStarted in the events file.

Verification:
- cargo test state -- --test-threads=1
- cargo test deepseek -- --test-threads=1

Expected Evidence:
- After this fix lands, the next trajectory should show `run_error_without_start` decreasing from 7 toward 0.
- New runs that crash before RunCompleted should appear in `state doctor` as "orphaned (closed by next session)" rather than "run_error_without_start."
- The orphan closer (`close_orphaned_run_if_needed`) should find and close more runs.

Implementation Notes:
- The fix has two parts:

  **Part 1: Reorder `ensure_run_started()` (line 500-509).**
  Move `RUN_STARTED.with(|c| c.set(true))` BEFORE the `record()` call. Currently the order is:
  ```rust
  fn ensure_run_started() {
      if !RUN_STARTED.with(|c| c.get()) {
          record(EventType::RunStarted, ...);  // calls record()
          RUN_STARTED.with(|c| c.set(true));   // set after
      }
  }
  ```
  Change to:
  ```rust
  fn ensure_run_started() {
      if !RUN_STARTED.with(|c| c.get()) {
          RUN_STARTED.with(|c| c.set(true));   // set BEFORE record()
          record(EventType::RunStarted, ...);
      }
  }
  ```
  This prevents infinite recursion when `ensure_run_started` is called from inside `record()` — the re-entrant `record()` call will see RUN_STARTED=true and skip the `ensure_run_started` check.

  **Part 2: Add `ensure_run_started()` call at the top of `record()` (line 447).**
  ```rust
  pub fn record(event_type: EventType, actor: Actor, payload: Value) {
      ensure_run_started();  // <-- add this line
      let guard = GLOBAL_RECORDER.lock().unwrap_or_else(|e| e.into_inner());
      let Some(recorder) = guard.as_ref() else {
          return;
      };
      // ... rest unchanged
  }
  ```
  When `ensure_run_started()` is called and RUN_STARTED is false:
  1. RUN_STARTED is set to true (Part 1)
  2. `ensure_run_started` calls `record()` for RunStarted
  3. Inside the re-entrant `record()` call, `ensure_run_started()` is called again
  4. RUN_STARTED is already true → no-op → no infinite recursion
  5. RunStarted is appended to the events file
  6. Control returns to the outer `record()` call, which appends the original event

- The `ensure_run_started()` call in `record()` fires on EVERY event emission. It's guarded by a thread-local `Cell<bool>` read (`RUN_STARTED`), so after the first call it's a single branch that costs essentially nothing.
- Do NOT modify `mark_run_completed` or `mark_run_completed_with_error` — they already call `ensure_run_started()` and the re-entrancy fix makes their existing calls safe (RUN_STARTED will already be true by the time RunCompleted is emitted, so `ensure_run_started` becomes a no-op).
- The existing `mark_run_completed_with_error_emits_retroactive_run_started_when_not_started` test expects a retroactive RunStarted to appear before RunCompleted. With this change, the retroactive RunStarted will appear even earlier (at the first `record()` call before RunCompleted). Update the test if it asserts on exact event ordering, but the test should still pass because the retroactive RunStarted still appears before RunCompleted.
