Title: Emit FailureObserved on agent error exit instead of relying on retroactive repair
Files: src/lib.rs
Issue: none
Origin: planner

Evidence:
- Assessment self-test: `yyds state why last-failure` shows retroactive FailureObserved with source=unknown, reason="retroactive: run completed with error status 'error' but no FailureObserved was recorded"
- 3 similar historical FailureObserved events exist (evt-harness-4da1aecc298af630, evt-harness-71db6103e66921ab, evt-harness-b85832ef85f2482f)
- `exit_with_state()` in src/lib.rs:985-993 calls `mark_run_completed_with_error()` (emits RunCompleted with status=error) but does NOT call `state::record(EventType::FailureObserved, ...)`
- `mark_run_completed_with_error()` at src/state.rs:583 correctly emits RunCompleted but the FailureObserved gap is in the caller, not the callee
- `scripts/append_terminal_state_events.py` retroactively fills this gap, but with source=unknown — the real exit code and error context are lost
- The panic hook at src/state.rs:60 already records FailureObserved for panics; the error-exit path should do the same

Edit Surface:
- src/lib.rs

Verifier:
- cargo test
- cargo build

Fallback:
- If `exit_with_state` is called from a code path that already emitted FailureObserved via the panic hook, don't worry about it — panics unwind and don't reach exit_with_state. No duplicate guard needed.
- If `state::take_diagnostic_error()` doesn't exist, use an empty string for error_detail or call the existing stash-read function.

Objective:
Eliminate retroactive FailureObserved events with source=unknown by having the agent process emit an explicit FailureObserved before exiting with a non-zero exit code.

Why this matters:
The retroactive repair pattern (scripts/append_terminal_state_events.py detecting error-exit runs with no FailureObserved) creates events with source=unknown that can't distinguish between API failures, crash exits, timeout kills, and intentional error exits. By emitting FailureObserved at the point of exit, the event carries the actual exit code and any stashed diagnostic error context. This makes state queries (`state why last-failure`) return actionable information instead of "source=unknown."

Success Criteria:
- `exit_with_state(code)` with code != 0 records a FailureObserved event with payload including the exit code and any stashed diagnostic error
- `exit_with_state(0)` does NOT record FailureObserved (clean exit)
- No duplicate FailureObserved events when exit_with_state is called from a path that already has one (e.g., if a panic hook fired first)
- `cargo test` passes with no regressions
- After landing, new error-exit runs show FailureObserved with non-unknown source in `yyds state why last-failure`

Verification:
- cargo test
- cargo build
- Manual: `cargo run -- state why last-failure` after a test error-exit (or rely on existing unit tests)

Expected Evidence:
- Next session with a non-zero exit produces a FailureObserved with source="agent_error_exit" (or similar), not source=unknown
- `yyds state why last-failure` shows the exit code in the failure payload
- The retroactive repair in append_terminal_state_events.py no longer needs to fill this gap for new runs

Implementation Notes:
- The change is in `src/lib.rs`, function `exit_with_state()` at line 985
- When `code != 0`, before calling `mark_run_completed_with_error()`, emit:
  ```
  state::record(
      state::EventType::FailureObserved,
      state::Actor::Harness,
      json!({"source": "agent_error_exit", "exit_code": code, "error_detail": state::take_diagnostic_error()}),
  );
  ```
- Check if `state::take_diagnostic_error()` exists — if not, use the existing stash mechanism or pass an empty string
- The `stash_diagnostic_error` function is called before `exit_with_state` in several places (e.g., line 869: `state::stash_diagnostic_error("api_error: piped_fallback_exhausted")`) — the FailureObserved should include that stashed message if available
- A thread-local duplicate guard is NOT needed: the panic hook (which emits FailureObserved) and error-exit are mutually exclusive — after a panic the process unwinds, it doesn't reach exit_with_state
- Keep the change under 20 lines total
- Do NOT modify src/state.rs in this task (task_01 already touches it)
