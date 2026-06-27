Title: Close state run lifecycle gap — emit RunCompleted for every RunStarted
Files: scripts/append_terminal_state_events.py, scripts/log_feedback.py
Issue: none
Origin: harness-seed (refined by planner from fresh trajectory evidence)

Evidence:
- Trajectory Day 119: `state_run_incomplete_count=1`, cause `state_incomplete/open_after_RunStarted=1`
- Log feedback corrected lesson: "state run lifecycle was incomplete: state_incomplete/open_after_RunStarted=1 → emit RunCompleted events for every started run, including timeout and API-error exits"
- State doctor confirms 57.9k events across 62 runs; one run lifecycle didn't close properly
- Assessment: "earlier runs show RunCompleted status=error events — indicating some runs terminated abnormally"
- This gap blocks accurate session success rate tracking (currently 0.0) and state capture coverage metrics

Edit Surface:
- scripts/append_terminal_state_events.py (terminal event recording logic)
- scripts/log_feedback.py (lifecycle gap detection and lesson generation)

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events scripts.test_task_lineage_feedback

Fallback:
- If all RunStarted events in the current state already have matching RunCompleted events, or if the gap is already fixed in the current code, write an obsolete_already_satisfied note with evidence.
- If the scripts don't contain the relevant lifecycle-gap logic, narrow the investigation to the actual owner.

Objective:
Ensure every RunStarted event has a corresponding RunCompleted event emitted, even when the session terminates via timeout, API error, or crash. This closes the `state_incomplete/open_after_RunStarted=1` lifecycle gap.

Why this matters:
The `state_run_incomplete_count` gnome directly blocks accurate session success rate measurement. When a RunStarted has no RunCompleted, the state system can't determine whether that session succeeded, failed, or crashed — making the `session_success_rate` metric unreliable. This is one of the trajectory's top-5 graph-derived pressure signals.

Success Criteria:
- The append_terminal_state_events module emits RunCompleted for every RunStarted, including timeout and API-error exit paths
- Log feedback correctly classifies pre-agent input-validation exits as non-lifecycle events (not as incomplete runs)
- Lifecycle lessons are emitted only for real incomplete or non-validation unmatched paths
- Existing tests continue to pass

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events scripts.test_task_lineage_feedback
- bash -n scripts/evolve.sh

Expected Evidence:
- Future trajectory snapshots show `state_run_incomplete_count` moving toward 0
- Future log feedback scores show improved state_capture coverage
- The corrected lesson "emit RunCompleted events for every started run" stops appearing as a recurring recommendation

Implementation Notes:
- The gap is `open_after_RunStarted` — a RunStarted was recorded but no matching RunCompleted followed. Look for exit paths in the session lifecycle that don't emit RunCompleted: timeouts, API errors, signal handlers, early exits.
- The assessment notes that input-validation exits should stay classified separately from non-validation unmatched completions. Don't conflate pre-flight validation exits (which are correct early exits) with actual lifecycle gaps.
- Keep the change scoped to the listed files. If the fix requires changes to evolve.sh or state.rs, narrow the scope instead — this task should address the Python-side lifecycle tracking, not the Rust event emission.
