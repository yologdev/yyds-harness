Title: Fix orphaned-run detection window to reliably close incomplete runs
Files: src/state.rs
Issue: none
Origin: planner

Evidence:
- Trajectory snapshot: state_run_incomplete_count=2, lifecycle cause "state_incomplete/open_after_SessionStarted=2"
- Log feedback correction: "emit RunCompleted events for every started run, including timeout and API-error exits"
- `close_orphaned_run_if_needed` in src/state.rs line 337 limits scan to last 20 events — sessions with >20 events after RunStarted have their orphans silently missed
- `state why last-failure` already detects incomplete runs; `close_orphaned_run_if_needed` is called at startup (line 410) but the scan window is too small to catch all orphans

Edit Surface:
- src/state.rs

Verifier:
- cargo build && cargo test --lib state -- --test-threads=1

Fallback:
- If the orphaned-run count in trajectory is already zero or the function already scans full file, mark task obsolete.

Objective:
Make `close_orphaned_run_if_needed` reliably detect and close orphaned runs regardless of how many events appear between RunStarted and the end of the events file.

Why this matters:
Two runs have RunStarted but no RunCompleted in state evidence. The log feedback explicitly calls for RunCompleted emission on every started run. The existing orphan-closure mechanism at startup is the right place to fix this, but its 20-event scan window is a blind spot for long sessions. Closing this gap improves state accuracy — downstream diagnostics, trajectory computation, and lifecycle claims all depend on complete run records.

Success Criteria:
- `close_orphaned_run_if_needed` finds RunStarted/RunCompleted even when they are more than 20 events from the file end
- Existing tests pass (cargo test --lib state)
- No regression in startup performance (the function already reads the full file; the change only affects how many events it examines)

Verification:
- cargo build
- cargo test --lib state -- --test-threads=1
- Add a unit test: create a temp events file with RunStarted + 25 intervening events + no RunCompleted, verify orphan closure fires
- Add a unit test: create a temp events file with RunStarted + RunCompleted within 5 events, verify no false orphan closure

Expected Evidence:
- Future trajectory snapshots should show decreasing state_run_incomplete_count
- State lifecycle claims should show fewer "open_after_SessionStarted" gaps
- `state doctor` should eventually report zero orphaned runs once existing orphans are retroactively closed

Implementation Notes:

The function `close_orphaned_run_if_needed` at line 308 of `src/state.rs` currently:

1. Reads the entire events file
2. Takes only the last 20 events (`events.len() - 20` to end)
3. Scans backward through those 20 for RunStarted/RunCompleted
4. If RunStarted is found first (no RunCompleted after it), emits RunCompleted("error")

The fix: change the scan to walk backward from the end until a lifecycle event (RunStarted or RunCompleted) is found, rather than stopping at 20 events. This is still efficient — it stops at the first lifecycle event encountered, and in the common case (successful session), that's RunCompleted within the last few events.

Specific change:
- Remove the `tail_start` / slice logic (lines 337-342)
- Change the `for event in tail.iter().rev()` loop (line 346) to iterate over the full events vec in reverse
- Add a break when the first RunStarted or RunCompleted is found (already present via `return Ok(())`/`break`)
- Update the doc comment on line 305 to reflect the new behavior

Also add unit tests:
- `test_close_orphaned_run_detects_distant_runstarted`: RunStarted + 25 non-lifecycle events, no RunCompleted → expect orphan closure
- `test_close_orphaned_run_no_false_positive`: RunStarted + 5 non-lifecycle events + RunCompleted → expect no orphan
- `test_close_orphaned_run_empty_file`: empty events file → Ok, no events emitted
- `test_close_orphaned_run_already_closed`: RunCompleted is the most recent lifecycle event → Ok, no orphan
