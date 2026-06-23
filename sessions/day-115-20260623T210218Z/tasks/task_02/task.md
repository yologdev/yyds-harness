Title: Emit RunCompleted from Rust panic hook
Files: src/state.rs
Issue: none
Origin: planner

Evidence:
- Graph-derived next-task pressure #1: "Close yyds state and model lifecycle gaps (state_run_incomplete_count=1): Lifecycle causes: state_incomplete/open_after_RunStarted=1"
- Log feedback corrective lesson: "state run lifecycle was incomplete: state_incomplete/open_after_RunStarted=1 -> emit RunCompleted events for every started run, including timeout and API-error exits"
- Assessment bug #4: "Lifecycle RunCompleted gap — certain exit paths not emitting RunCompleted"
- State aggregate: run_incomplete=118, model_incomplete=54
- Code evidence: `install_panic_hook()` (src/state.rs:31-57) records `FailureObserved` but does NOT emit `RunCompleted`. If a Rust panic terminates a session, the run stays open forever (until orphan detection retroactively closes it on next start).
- `mark_run_completed_with_error()` (src/state.rs:486) exists and is callable from the panic hook context — it uses the same `record()` path already in use.

Edit Surface:
- src/state.rs (install_panic_hook function only, ~lines 31-57)

Verifier:
- cargo test -- test_panic_hook_records_run_completed
- cargo test -- state

Fallback:
- If the panic hook cannot safely emit RunCompleted (e.g., the global recorder is not initialized at the time of panic), document the limitation in a code comment and rely on orphan detection as the catch-up mechanism. Do not refactor the panic hook architecture.

Objective:
Ensure that a Rust panic during an agent session emits a RunCompleted(error) event so the run lifecycle is properly closed, reducing the run_incomplete count in state health metrics.

Why this matters:
The state lifecycle gap (run_incomplete=118 aggregate) is the #1 graph-derived pressure item. Every incomplete run means state health tools see a dangling RunStarted with no matching RunCompleted, which pollutes diagnostics and makes it harder to distinguish "session still in progress" from "crashed session." The orphan detection in src/state.rs (added Day 114) retroactively fixes old gaps, but preventing new gaps at the source (the panic hook) is more reliable.

Success Criteria:
- install_panic_hook() emits a RunCompleted("error") event (via mark_run_completed_with_error) before calling the previous hook
- The emission is fail-soft: if the global recorder isn't initialized, the panic hook still calls prev_hook and the process still terminates
- A new unit test verifies that a panic in a thread with state initialized produces a RunCompleted event in the events file

Verification:
- cargo build
- cargo test -- state
- cargo test -- test_panic_hook

Expected Evidence:
- Future state doctor runs show fewer run_incomplete counts
- State lifecycle gnome (state_incomplete) decreases
- Task lineage shows RunCompleted events for sessions that terminated via Rust panic

Implementation Notes:
- Add the RunCompleted emission inside the panic hook closure, AFTER the existing FailureObserved record and BEFORE prev_hook(info)
- Use mark_run_completed_with_error("rust_panic") — this already exists and is tested
- Wrap the call in a fail-soft guard (let _ = std::panic::catch_unwind or similar) so a double-panic doesn't prevent prev_hook from running
- Add a unit test: initialize state, spawn a thread that panics, read back the events file, assert RunCompleted is present
- The existing test `panic_hook_records_to_state` (line 6657) can serve as a template
- Do NOT modify scripts/evolve.sh (protected file)
