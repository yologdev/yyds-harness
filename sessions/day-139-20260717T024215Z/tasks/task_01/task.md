Title: Deduplicate retroactive FailureObserved events for runs that already have one
Files: scripts/append_terminal_state_events.py
Issue: none
Origin: planner

Evidence:
- `yyds state why last-failure` shows 5 retroactive FailureObserved events for the same cancelled run (trace-evolve-29489777915), each with payload `"run completed with error status 'error' but no FailureObserved was recorded"`.
- The existing test `test_skips_failure_observed_when_already_present` covers the case where FailureObserved was already present BEFORE the run's terminal events were appended, but it does NOT cover the case where a retroactive FailureObserved was already emitted by a previous invocation of the script against the same events file.
- The terminal-state script (`append_terminal_state_events.py`) can be invoked multiple times against the same events file (e.g., from multiple post-processing hooks or retries), and each invocation re-scans the file for incomplete runs and emits a new retroactive FailureObserved if it doesn't find one — but it doesn't check whether a *retroactive* FailureObserved was already emitted by a prior invocation.

Edit Surface:
- scripts/append_terminal_state_events.py

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events.AppendTerminalStateEvents.test_skips_failure_observed_when_already_present

Fallback:
- If the dedup test already covers the retroactive path and the duplicate events are caused by something outside this script (e.g., the harness calling the script concurrently), write a brief diagnostic note instead of editing.

Objective:
Ensure that each run gets at most one retroactive FailureObserved event, even when the terminal-state script is invoked multiple times against the same events file.

Why this matters:
Duplicate FailureObserved events inflate `deepseek_model_call_incomplete_count` and related gnome metrics with phantom signals. Each duplicate makes the state graph think there's a new failure when there isn't, which feeds noise into trajectory-based task selection and wastes implementation slots on already-fixed issues.

Success Criteria:
- After the fix, a second invocation of `append_terminal_events` against an events file that already has a retroactive FailureObserved for a given run does NOT emit another one.
- The existing test `test_skips_failure_observed_when_already_present` continues to pass.
- A new test (or extended existing test) verifies the dedup across multiple invocations.

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events

Expected Evidence:
- Future `state why last-failure` output shows at most 1 retroactive FailureObserved per run.
- `deepseek_model_call_incomplete_count` stabilizes (stops growing from duplicate retroactive events).

Implementation Notes:
- The fix should be in the `append_terminal_events` function in `scripts/append_terminal_state_events.py`.
- When scanning for incomplete runs that need a retroactive FailureObserved, also scan for existing retroactive FailureObserved events (those with `"retroactive": true` in the payload) for the same run_id and skip the run if one already exists.
- Add a test that: writes events for an error run without FailureObserved, runs append_terminal_events once (which emits retroactive FailureObserved), then runs it again against the same file and verifies no second retroactive FailureObserved was appended.
