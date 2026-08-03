Title: Close orphaned model call lifecycles at startup alongside orphaned runs
Files: src/state.rs
Issue: none
Origin: planner

Evidence:
- Trajectory shows deepseek_model_call_abnormal_completed_count=1 with lifecycle causes including state_unmatched/open_after_FailureObserved=8
- Recent state events show ModelCallCompleted with status "timeout" and "interrupted" — abnormal completions that may not pair cleanly with ModelCallStarted
- The panic hook (src/state.rs lines 64-74) already closes in-flight model calls on panic by reading CURRENT_MODEL_CALL_ID and emitting ModelCallCompleted with status "interrupted"
- close_orphaned_run_if_needed (src/state.rs line 360) already detects and closes orphaned runs (RunStarted without RunCompleted) at startup, but does not check for orphaned model calls (ModelCallStarted without ModelCallCompleted) within those runs
- When a process is SIGKILL'd or crashes outside the panic hook path, ModelCallStarted may be left dangling with no matching ModelCallCompleted — the script-level doctor (append_terminal_state_events.py) can catch these post-hoc, but the Rust-level startup check is the first line of defense
- This is a Rust change verifiable by cargo test, avoiding the evaluator-timeout revert risk

Edit Surface:
- src/state.rs

Verifier:
- cargo test state

Fallback:
- If the events file format makes it difficult to reliably pair ModelCallStarted with ModelCallCompleted by model_call_id (the payload key), narrow to only closing model calls that share the same run_id as the orphaned run being closed. If even that is unreliable, add a test proving the current behavior is correct and mark complete.

Objective:
Extend close_orphaned_run_if_needed to also detect and close orphaned model calls: when an orphaned run is found (RunStarted without RunCompleted), scan for any ModelCallStarted events within that run that lack a matching ModelCallCompleted, and emit the missing ModelCallCompleted with status "interrupted" and error_detail "orphaned_at_startup".

Why this matters:
The model call lifecycle gap (deepseek_model_call_abnormal_completed_count=1) produces noise in state gnomes and makes it harder to distinguish real DeepSeek API failures from process-kill artifacts. Closing these gaps at startup (Rust-level, fast, synchronous) is the first line of defense; the Python script-level doctor is the second. Both layers are needed because SIGKILL prevents the panic hook from firing.

Success Criteria:
- close_orphaned_run_if_needed detects ModelCallStarted events without matching ModelCallCompleted within orphaned runs and emits the missing ModelCallCompleted
- The emitted ModelCallCompleted has status "interrupted" and error_detail "orphaned_at_startup" to distinguish from panic-hook closures
- Existing tests in src/state.rs continue to pass, including close_orphaned_run_detects_distant_runstarted
- A new test proves the model-call closing behavior

Verification:
- cargo test state
- cargo build

Expected Evidence:
- deepseek_model_call_abnormal_completed_count decreases in future trajectory snapshots
- state doctor shows fewer unmatched model call lifecycle events
- The new test (close_orphaned_run_closes_model_calls or similar) appears in test output

Implementation Notes:
- The function currently scans events backward from the end, finds the most recent RunStarted/SessionStarted without RunCompleted, and emits a retroactive RunCompleted.
- To extend it: after finding the orphaned run_id, scan all events for ModelCallStarted entries matching that run_id that lack a corresponding ModelCallCompleted. Emit ModelCallCompleted for each orphaned model call.
- Use the same model_call_terminal_payload helper (or a simplified inline payload) with status="interrupted" and error_detail="orphaned_at_startup".
- Keep the change minimal — don't refactor the function signature. The model call closing should happen between the orphan run detection and the RunCompleted emission.
- The model_call_id for the closure event can be extracted from the orphaned ModelCallStarted's payload.
- If no model_call_id is available in the payload, use the run_id as a fallback identifier.
