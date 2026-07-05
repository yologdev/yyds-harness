Title: Append missing FailureObserved events for error-completed runs
Files: scripts/append_terminal_state_events.py, scripts/test_append_terminal_state_events.py
Issue: none
Origin: planner

Evidence:
- `state why last-failure` reports 78 sessions completed with errors but no FailureObserved events recorded
- `state lifecycle --limit 1000` shows lifecycle imbalance: runs completed via error path without formal start records (`run_error_without_start=8`)
- The `build_why_report` function in `src/commands_state.rs` (line 3220-3229) already detects and reports this gap: "N sessions completed with errors but no FailureObserved events were recorded"
- `scripts/append_terminal_state_events.py` already detects and closes orphaned runs (runs started but never completed); it should also detect runs completed with error status that lack FailureObserved
- Day 126 Task 1 fixed single-session-scope orphaned-run detection in this script; this task extends it to the FailureObserved gap

Edit Surface:
- scripts/append_terminal_state_events.py — add detection for runs with RunCompleted(error) but no matching FailureObserved, and append a FailureObserved event
- scripts/test_append_terminal_state_events.py — add test cases for the new detection

Verifier:
- python3 -m pytest scripts/test_append_terminal_state_events.py -v

Fallback:
- If `append_terminal_state_events.py` already handles this case or the test fixtures don't support it, write a diagnostic note and mark the task as done-with-findings instead of forcing a code change.

Objective:
Close the FailureObserved event gap so that error-completed sessions are properly tracked in the state event log, making `state why last-failure` and lifecycle queries accurate.

Why this matters:
78 sessions completed with errors but never recorded a FailureObserved event. This means failure diagnostics (`state why last-failure`, lifecycle queries, trajectory analysis) are blind to real failures that happened. Each missing FailureObserved is a session where something went wrong and the evidence system didn't capture it — the same class of bug that Days 114-115 taught us about ("crash boundaries are where evidence goes to die").

This closes one of the graph-derived next-task pressure items: "Close yyds state and model lifecycle gaps (`run_error_without_start=8`)."

Success Criteria:
- `append_terminal_state_events.py` detects runs where RunCompleted payload status is not "success"/"completed" and no FailureObserved event exists for that run_id
- A FailureObserved event is appended with run_id, timestamp, and a payload explaining the error completion
- Test coverage validates both the detection and the append path
- No false positives: runs that already have FailureObserved are not double-counted

Verification:
- python3 -m pytest scripts/test_append_terminal_state_events.py -v
- Verify the script runs without errors on existing state data

Expected Evidence:
- State lifecycle: incomplete/error runs without FailureObserved count decreases
- `state why last-failure`: the "no FailureObserved events were recorded" message disappears for retroactively-fixed runs
- Dashboard: failure event coverage improves

Implementation Notes:
- The RunCompleted event's payload has a `status` field. Values like "error", "run_error_without_start", or any non-"success"/"completed" value signal an error completion.
- The FailureObserved event should include: event_type="FailureObserved", run_id matching the error RunCompleted, a timestamp (use the RunCompleted timestamp), and a payload with `reason: "retroactive: run completed with error status '<status>' but no FailureObserved was recorded"`.
- Follow the existing pattern in the script for appending events (use `json.dumps()` via python3, never echo).
- Only scan runs that completed with errors — don't scan every run.
- The test file already has fixtures for orphaned run detection; add similar fixtures for the FailureObserved gap.
- This script is NOT in PROTECTED_IMPLEMENTATION_FILES and is safe to modify.
