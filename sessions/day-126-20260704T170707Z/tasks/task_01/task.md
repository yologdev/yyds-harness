Title: Fix orphaned-run detection gap in terminal-state script
Files: scripts/append_terminal_state_events.py, scripts/test_append_terminal_state_events.py
Issue: none
Origin: planner

Evidence:
- State doctor shows 1 incomplete run: `github-actions-28319290130`, started ~6.3 days ago, no RunCompleted event. Cause: `state_incomplete/open_after_SessionStarted=1`.
- Graph-derived pressure row 1: "Close yyds state and model lifecycle gaps (state_run_incomplete_count=1): Lifecycle causes: state_incomplete/open_after_SessionStarted=1"
- Assessment: "The terminal-state script's orphan detector may not be catching single-session-scoped runs from cancelled GitHub Actions. Need to verify whether Day 124's fix actually handles this case or whether this run predates that fix."
- `yyds state why last-failure` confirms: "1 incomplete run: github-actions-28319290130, started 9,038 minutes ago (~6.3 days), no RunCompleted event. This is an orphaned run — likely from a GitHub Actions cancellation."

Edit Surface:
- scripts/append_terminal_state_events.py (orphan detection logic)
- scripts/test_append_terminal_state_events.py (test for orphan detection)

Verifier:
- python3 -m pytest scripts/test_append_terminal_state_events.py -v
- python3 scripts/append_terminal_state_events.py --help

Fallback:
- If the orphaned run predates Day 124's fix and the current code would catch a fresh orphan, write a test proving the detection works for the current code path, note that the existing orphan is pre-fix, and mark the task done.
- If the detection already works correctly and the orphan is truly pre-fix, narrow the task to adding a test that proves the detection works for cancelled-GitHub-Actions scenarios.

Objective:
Ensure the terminal-state script detects and closes orphaned runs (RunStarted without RunCompleted) that result from GitHub Actions cancellations, so the state lifecycle stays clean and `state_run_incomplete_count` drops to zero.

Why this matters:
Orphaned runs create noise in state diagnostics (state doctor, state why last-failure) and inflate state_run_incomplete_count. When the harness can't distinguish "run still in progress" from "run killed externally 6 days ago," every diagnostic that counts incomplete runs reports stale data. The graph-derived pressure explicitly calls this out as the #1 priority. Fixing it also improves state_capture_coverage — the diagnostic gnome that tracks how complete our lifecycle recording is.

Success Criteria:
- The orphan detection in append_terminal_state_events.py recognizes runs with RunStarted but no RunCompleted (or RunCompletedWithError) and closes them with an appropriate terminal event.
- A test in test_append_terminal_state_events.py verifies that a simulated orphaned run (RunStarted only, no terminal event) gets detected and closed.
- The existing orphaned run github-actions-28319290130 is either (a) caught by the updated detection or (b) confirmed as pre-fix and documented as such.

Verification:
- python3 -m pytest scripts/test_append_terminal_state_events.py -v
- python3 scripts/append_terminal_state_events.py --help

Expected Evidence:
- state_run_incomplete_count drops to 0 after the fix runs (or the existing orphan is documented as pre-fix).
- New test passes: a simulated orphaned run is detected and closed.
- No regression in existing test_append_terminal_state_events.py tests.

Implementation:
1. Read scripts/append_terminal_state_events.py to understand the current orphan detection logic, especially the Day 124 fix for single-session-scoped runs.
2. Identify why github-actions-28319290130 wasn't caught: is it a scope mismatch (session vs. pipeline run), a timing issue (script runs before RunCompleted arrives), or a detection gap in the current logic?
3. Add or fix the detection: the script should scan for any RunStarted event that has no matching RunCompleted/RunCompletedWithError within the same run_id, and emit a RunCompletedWithError (or equivalent terminal event) with a cause like "orphaned/github_actions_cancellation".
4. Add a test case: create a synthetic event list with one RunStarted and no terminal event, assert the script detects and closes it.
5. Run the existing test suite to confirm no regressions.
