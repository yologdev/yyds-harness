Title: Close historical state lifecycle gaps — retroactively add FailureObserved for error runs
Files: scripts/append_terminal_state_events.py
Issue: none
Origin: planner

Evidence:
- Graph pressure: `state_run_unmatched_non_validation_completed_count=2` — 2 sessions completed with error status but no FailureObserved events.
- Assessment: `yyds state why last-failure` reports "3 error runs without FailureObserved events" and "1 incomplete run (13,938 minutes stale)."
- Assessment self-tests: `yyds state doctor` confirms 107K events, SQLite healthy, but lifecycle gaps persist.
- The `find_missing_failure_observed()` function in `scripts/append_terminal_state_events.py` already detects these gaps — it finds runs with error-status RunCompleted events that lack matching FailureObserved events. The script was built on Day 127 but historical gaps remain because it only handles new occurrences.

Edit Surface:
- scripts/append_terminal_state_events.py

Verifier:
- python3 scripts/append_terminal_state_events.py --dry-run 2>&1 | grep -c "missing_failure_observed"
- If gaps found: the script should emit FailureObserved events for each gap and the count should drop to 0 on re-run.
- If no gaps found (already fixed): verify the script exits 0 and reports "0 missing" — task is already resolved, mark it done.

Fallback:
- If the script already handles all gaps and `find_missing_failure_observed` returns 0 on the current events file, write an obsolete-task note explaining the gaps were already closed between assessment and planning.

Objective:
Ensure `scripts/append_terminal_state_events.py` retroactively closes all historical lifecycle gaps — every error-status RunCompleted event has a matching FailureObserved event, and every stale incomplete run has a terminal event. After this task, `yyds state why last-failure` should report 0 error runs without FailureObserved.

Why this matters:
The trajectory graph pressure `state_run_unmatched_non_validation_completed_count=2` means the state system has blind spots in its failure-tracking. These gaps make it impossible to distinguish "the session failed and we recorded it" from "the session failed and the evidence was lost." Closing them makes failure analysis trustworthy, which directly improves the harness's ability to learn from its own failures. This is a diagnostic gate — not a fitness gnome — but it's blocking accurate failure measurement.

Success Criteria:
- The `append_terminal_state_events.py` script is run (or fixed and run) against the current events file.
- All error-status RunCompleted events now have matching FailureObserved events.
- All stale incomplete runs (started >24h ago with no RunCompleted) now have terminal events.
- `yyds state why last-failure` shows 0 (or fewer) error runs without FailureObserved.
- The change touches only `scripts/append_terminal_state_events.py` (if a code fix is needed) or is a no-op if gaps were already closed.

Verification:
- python3 scripts/append_terminal_state_events.py --dry-run 2>&1
- yyds state why last-failure 2>&1
- Check that error runs without FailureObserved moves toward 0.

Expected Evidence:
- `state_run_unmatched_non_validation_completed_count` gnome drops from 2 toward 0.
- `yyds state tail --limit 50` shows newly appended FailureObserved events with matching run_ids.
- Task lineage shows `scripts/append_terminal_state_events.py` in the changed files.

Detailed description:

The script `scripts/append_terminal_state_events.py` was built on Day 127 to retroactively close lifecycle gaps. Its `find_missing_failure_observed()` function scans the events file for RunCompleted events with error status that lack matching FailureObserved events. Its `find_stale_orphaned_runs()` finds runs that started but never completed. The `append_terminal_events()` function writes the missing events.

The current evidence shows 3 error runs without FailureObserved and 1 incomplete run. The implementation agent should:

1. Read `scripts/append_terminal_state_events.py` to understand the current logic.
2. Run it in dry-run mode (or add a --dry-run flag if missing) to see how many gaps exist.
3. If the script already handles all detected gaps: run it for real to append the missing events, then verify the gaps are closed.
4. If the script has a bug (doesn't detect some gaps, or doesn't write events correctly): fix the bug, add a test in `scripts/test_append_terminal_state_events.py` (if it exists), then run it.
5. Verify with `yyds state why last-failure` and `yyds state doctor`.

Keep changes to `scripts/append_terminal_state_events.py` only. If the script needs a test, the test file path is `scripts/test_append_terminal_state_events.py` — this is a separate file and should NOT be included unless the task explicitly says it's needed (it's not in Files: for this task).
