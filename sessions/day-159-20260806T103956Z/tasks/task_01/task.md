Title: Close model call lifecycle on InputRejected and other unclosed paths
Files: src/prompt.rs
Issue: none
Origin: planner (refined from harness-seed)

Evidence:
- Trajectory gnome: `deepseek_model_call_incomplete_count=1` with `open_after_ModelCallStarted=1` — one model call started but never completed.
- `handle_prompt_events` at line 782 records `ModelCallStarted` before the event loop. Three paths close it: normal `Completion` (line ~955), ctrl+c interrupt (line 1040), and channel-close cleanup (line 1101).
- The `AgentEvent::InputRejected` handler (line 977) records `FailureObserved` but does NOT record `ModelCallCompleted`. If the channel doesn't close immediately after rejection, or if yoagent holds the channel open expecting a retry, the `ModelCallStarted` from line 782 stays open indefinitely.
- Day 159 Task 1 closed the panic-hook gap (process crashes now close model calls), but non-panic paths like InputRejected remain unclosed.
- Assessment confirms: "stream errors, API timeouts, and abnormal completions can also leave ModelCallStarted without ModelCallCompleted without triggering a panic."

Edit Surface:
- src/prompt.rs (the InputRejected handler at ~line 977, and any other early-return/error paths that record ModelCallStarted but not ModelCallCompleted)

Verifier:
- cargo build
- cargo test --lib state::tests -- prompt::tests

Fallback:
- If `cargo build` or `cargo test` fail on unrelated code, scope the fix to only the InputRejected handler and do not touch other event handlers.
- If the gap is proven to be historical (pre-Day-159) and no longer reproducible, write an obsolete note.

Objective:
Ensure every `ModelCallStarted` recorded in `handle_prompt_events` has a matching `ModelCallCompleted`, so the `deepseek_model_call_incomplete_count` gnome converges to zero for new sessions.

Why this matters:
An incomplete model call lifecycle means the state doctor and trajectory extractor see a "ghost" call that was started but never finished. This pollutes lifecycle gnomes and causes the harness to keep re-diagnosing the same gap session after session. Closing this gap makes each session's state evidence trustworthy.

Success Criteria:
- The `InputRejected` handler records `ModelCallCompleted` with status "input_rejected" (or similar) before returning/continuing.
- Any other event handlers that record only `ModelCallStarted` without a matching `ModelCallCompleted` are also fixed, or documented as intentionally left open with a clear comment.
- `cargo build && cargo test` pass.

Verification:
- cargo build
- cargo test --lib state::tests -- prompt::tests
- cargo fmt -- --check

Expected Evidence:
- Future trajectory blocks show `deepseek_model_call_incomplete_count=0` for sessions after this fix lands.
- State doctor no longer reports orphaned `ModelCallStarted` events from post-fix sessions.

Implementation Notes:
- The fix is surgical: in the `AgentEvent::InputRejected` arm (around line 977), after recording `FailureObserved`, record a `ModelCallCompleted` event with status "input_rejected" and call `clear_current_model_call_id()`.
- Set `model_call_terminal_recorded = true` so the channel-close cleanup at line 1092 doesn't double-record.
- If `model_call_started` is false (shouldn't happen since line 782 always sets it), guard with `if !model_call_started { record ModelCallStarted first }` — same pattern used in the ctrl+c handler at line 1032.
- Do NOT modify the `FailureObserved` recording or the diagnostic print — only ADD the lifecycle closure.
- This is a ~10-line addition in a single match arm.
