Title: Add regression test for find_missing_model_call_started in append_terminal_state_events.py
Files: scripts/test_append_terminal_state_events.py
Issue: #155
Origin: planner (refined from harness-seed; narrowed from 3 files to 1 test-only change)

Evidence:
- #155 reverted because agent spent 24 turns analyzing existing code without producing any edits. The feature (find_missing_model_call_started at scripts/append_terminal_state_events.py:436) already exists and correctly creates synthetic ModelCallStarted events for orphaned ModelCallCompleted events — but has zero regression tests.
- Structured state snapshot: model_incomplete/open_after_ModelCallStarted=8 — these lifecycle gaps from cancelled runs need post-hoc repair. The repair logic exists; what's missing is a test that proves it works and prevents future regressions.
- Day 154 partially addressed lifecycle gaps (input-validation filtering + panic-path close), Day 156 11:23 cancelled run still produces retroactive events. The script-level repair is the right layer for SIGTERM-killed sessions.

Edit Surface:
- scripts/test_append_terminal_state_events.py (add one test function only — no changes to the production script)

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events

Fallback:
- If the existing test infrastructure doesn't support testing find_missing_model_call_started in isolation (e.g., it requires real state event files that aren't available in the test environment), write the test skeleton with a skip decorator and a comment explaining what fixture is needed. Do NOT modify scripts/append_terminal_state_events.py itself.

Objective:
Add one unittest that verifies find_missing_model_call_started correctly detects orphaned ModelCallCompleted events and that the main append function creates synthetic ModelCallStarted events for them.

Why this matters:
The find_missing_model_call_started repair logic has been in production for multiple sessions without test coverage. When it breaks (wrong filter, missed edge case), the lifecycle gaps silently grow and gnome KPIs become untrustworthy. A single test locks in the existing behavior and makes future changes to the lifecycle repair path safe.

Success Criteria:
- One new test function in scripts/test_append_terminal_state_events.py that imports and calls find_missing_model_call_started with a fixture containing one ModelCallCompleted without a matching ModelCallStarted.
- The test asserts that the function returns the orphaned completion in its result.
- All existing tests continue to pass.

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events

Expected Evidence:
- Test coverage report shows find_missing_model_call_started is now tested.
- Future changes to append_terminal_state_events.py that break the repair logic will be caught by this test.

Implementation Notes:
- find_missing_model_call_started is at scripts/append_terminal_state_events.py:436. Read its signature and return type before writing the test.
- The test file is scripts/test_append_terminal_state_events.py. Follow the existing test patterns in that file (use unittest.TestCase, construct event fixtures as dicts, call the function, assert on the result).
- The fixture should be minimal: one ModelCallCompleted event with no matching ModelCallStarted. The function should detect it as orphaned.
- If the existing tests use helper functions to construct events, use those helpers.
- This is a TEST-ONLY change. Do NOT modify scripts/append_terminal_state_events.py. The feature already works; we are only adding the missing test.
- Timebox: 15 minutes. If you can't get the test working in 15 minutes, write a skipped test with a comment explaining what's needed.
