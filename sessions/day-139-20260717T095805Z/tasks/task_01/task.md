Title: Deduplicate retroactive FailureObserved events across multiple script invocations
Files: scripts/append_terminal_state_events.py, scripts/test_append_terminal_state_events.py
Issue: #111
Origin: planner

Evidence:
- `yyds state why last-failure` has previously shown duplicate retroactive FailureObserved events for the same run, each with payload `"run completed with error status 'error' but no FailureObserved was recorded"`.
- The existing test `test_skips_failure_observed_when_already_present` covers the case where an original FailureObserved was already in the events file before the script runs. It does NOT cover the case where a retroactive FailureObserved was emitted by a prior invocation of the script against the same events file.
- `find_missing_failure_observed()` scans all events and checks for ANY FailureObserved per run_id — so sequential invocations should already be safe. But the test doesn't prove it, and concurrent invocations (two harness hooks firing simultaneously) could still race.
- Adding a test for multi-invocation dedup increases confidence that duplicate retroactive events won't inflate `deepseek_model_call_incomplete_count` and related gnome metrics.

Edit Surface:
- scripts/append_terminal_state_events.py
- scripts/test_append_terminal_state_events.py

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events.AppendTerminalStateEvents.test_skips_retroactive_failure_observed_on_second_invocation

Fallback:
- If the existing `find_missing_failure_observed` already handles the dedup correctly and the test passes on first attempt, the task is done — adding test coverage is the deliverable. If the function does NOT handle it (duplicate is emitted), fix `find_missing_failure_observed` to also check for retroactive FailureObserved events with `"retroactive": true` in the payload. If the function already works, the test proves it.

Objective:
Ensure each run gets at most one retroactive FailureObserved event, even when `append_terminal_events` is invoked multiple times against the same events file.

Why this matters:
Duplicate FailureObserved events inflate `deepseek_model_call_incomplete_count` and related gnome metrics with phantom signals. Each duplicate feeds noise into trajectory-based task selection and wastes implementation slots on already-diagnosed issues.

Success Criteria:
- A new test `test_skips_retroactive_failure_observed_on_second_invocation` passes: writes error-run events without FailureObserved, invokes `append_terminal_events` once (which emits retroactive FailureObserved), then invokes it again and verifies no second retroactive FailureObserved was appended.
- The existing test `test_skips_failure_observed_when_already_present` continues to pass.
- All other existing tests continue to pass.

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events

Expected Evidence:
- Future `state why last-failure` output shows at most 1 retroactive FailureObserved per run.
- `deepseek_model_call_incomplete_count` gnome stabilizes (stops growing from duplicate retroactive events).
- Task lineage shows test_added evidence.

Implementation Notes:
- The fix (if needed) goes in `find_missing_failure_observed()` in `scripts/append_terminal_state_events.py`. When building `failure_observed_runs`, also check for events with `"retroactive": true` in the payload — or simply continue relying on the existing check (any FailureObserved for the run_id counts).
- The new test should use the same pattern as `test_skips_failure_observed_when_already_present`: create events, call `append_terminal_events` twice, verify no duplicate.
- Do NOT edit only the test file — both files are in scope. If the main script needs a code fix, make it. If the main script already handles dedup correctly, the test alone is the deliverable.
