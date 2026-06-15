Title: Close RunCompleted lifecycle gap — emit correct status on panic exits
Files: src/state.rs
Issue: none
Origin: planner

Objective:
Ensure every RunStarted event gets a matching RunCompleted event with the correct status ("error" for panics, not "completed"). The `RunCompletionGuard` currently always reports "completed" on drop, even after a panic. This leaves lifecycle gaps where runs appear to succeed despite crashes.

Why this matters:
The trajectory shows `state_incomplete/open_after_SessionStarted=1` and the log feedback corrected lesson says "emit RunCompleted events for every started run, including timeout and API-error exits." Graph pressure #1 identifies this as the top lifecycle gap. The guard drop on panic currently records "completed" — masking real failures in state/dashboard.

Success Criteria:
- On normal exit, RunCompleted status is "completed" (unchanged)
- On panic, RunCompleted status is "error" with error detail
- On explicit exit_with_state, behavior unchanged (already reports correct status)
- No double RunCompleted events

Verification:
- cargo test state
- cargo test -- --test-threads=1
- cargo build

Expected Evidence:
- After deploy: state lifecycle shows no unmatched RunStarted events in fresh runs
- Dashboard lifecycle aggregate: run_incomplete count decreases
- Graph pressure #1: state_incomplete/open_after_SessionStarted resolves

Implementation Notes:

The fix is in `src/state.rs` and touches three things:

1. Add a thread-local flag `RUN_HAD_ERROR: Cell<bool>` (or `RefCell<bool>`) that tracks whether this run encountered a fatal error.

2. In `mark_run_completed_with_error`, set `RUN_HAD_ERROR` to true BEFORE recording the RunCompleted event (so the guard can see it, though in the exit_with_state path the guard never drops).

3. In `RunCompletionGuard::drop`, check `RUN_HAD_ERROR`: if true, call `mark_run_completed("error")` instead of `mark_run_completed("completed")`.

4. In `install_panic_hook`, set `RUN_HAD_ERROR` to true (so the guard picks it up when unwinding).

The thread-local should be declared like:
```rust
std::thread_local! {
    static RUN_HAD_ERROR: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
```

This ensures:
- Normal exit → guard drops → RUN_HAD_ERROR is false → "completed" ✓
- Panic → hook sets RUN_HAD_ERROR=true → guard drops → "error" ✓  
- exit_with_state → mark_run_completed_with_error → exit → guard never drops ✓ (unchanged)
- Signal kill → no code runs → RunCompleted never emitted (known limitation, not fixable in-process)

The panic hook at line 30 already records FailureObserved. Add one line to set RUN_HAD_ERROR:
```rust
RUN_HAD_ERROR.with(|c| c.set(true));
```

The guard's Drop impl changes from:
```rust
mark_run_completed(self.status);
```
to:
```rust
let status = if RUN_HAD_ERROR.with(|c| c.get()) { "error" } else { self.status };
mark_run_completed(status);
```
