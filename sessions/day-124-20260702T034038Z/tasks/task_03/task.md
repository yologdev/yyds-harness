Title: Make `append_terminal_state_events.py` detect and close session-scope orphaned runs
Files: scripts/append_terminal_state_events.py
Issue: #53
Origin: planner

Evidence:
- Issue #53 agent identified the root cause on Turn 19: `lifecycle_for_scope` explicitly skips session runs, so session-scope orphaned runs (e.g., from evaluator timeout) are never closed
- Trajectory: evaluator_unverified_count=2 — evaluator process timed out without closing the run lifecycle
- Trajectory: state_run_incomplete_count was flagged as "state_incomplete/open_after_SessionStarted" — the top-level session run stays permanently open
- The evaluator path IS handled for non-session runs (it uses `run_agent_with_completion_watch` → `record_agent_terminal_events`) — the gap is specifically session-scope runs that `lifecycle_for_scope` skips

Edit Surface:
- scripts/append_terminal_state_events.py (lifecycle_for_scope and/or the main closure loop; ~50 lines)

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events
- python3 scripts/append_terminal_state_events.py --help

Fallback:
- If the session-scope skip is intentional (e.g., session runs should only be closed by the harness itself), move the closure logic into evolve.sh's session-end path instead, and write an obsolete note for this task.
- If the script already handles this case and the #53 agent was wrong, verify with a dry-run and write an obsolete note.
- If no orphaned session runs exist in current state, write a unit test covering the case and mark the task done.

Objective:
Make `append_terminal_state_events.py` detect session-scope runs that have `RunStarted` but no `RunCompleted` and append a `RunCompleted` with `outcome: "post_hoc_closed"`. This is a narrow fix: only close session-scope orphans, not a full robustness rewrite.

Why this matters:
The evaluator timeout problem has a self-referential quality: the evaluator (which verifies tasks) times out, leaving orphaned runs, which inflate `state_run_incomplete_count`, which feeds into trajectory pressure, which tells the planner to fix lifecycle problems — but the planner can't fix them because the evaluator times out on those tasks too. Breaking this cycle requires fixing the post-hoc lifecycle closer so it detects and closes session-scope orphans. Once runs are properly closed, the state graph accurately reflects reality, and the planner can focus on real problems.

This also unblocks the `state_capture_coverage` gnome: if incomplete runs are inflating the count, the gnome is measuring infrastructure noise rather than harness quality.

Success Criteria:
- `append_terminal_state_events.py` detects session-scope runs where `RunStarted` exists but no `RunCompleted` or `FailureObserved` terminal event
- Appends a `RunCompleted` with a clear outcome field indicating post-hoc closure (e.g., `"post_hoc_closed"`)
- Does NOT close runs that already have a terminal event (no double-closing)
- Existing tests pass; new test covers the session-scope orphan case
- The change is ~30 lines or fewer in the script itself

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events
- python3 scripts/append_terminal_state_events.py --help (confirms script parses)
- Dry-run on any available state data to confirm session-scope detection works

Expected Evidence:
- Future structured state snapshots show `state_run_incomplete_count` decreasing
- The trajectory extractor no longer flags lifecycle gaps from evaluator-timeout orphans
- The `evaluator_unverified_count` pressure signal can be traced to actual evaluator failures rather than stale state

Implementation:
1. Read `scripts/append_terminal_state_events.py`. Focus on two things:
   a. `lifecycle_for_scope` — where it skips session scope (look for `"session"` in the scope filter)
   b. The main loop that iterates open runs and calls the closure logic
2. The fix: either add session scope to `lifecycle_for_scope`'s allowed scopes, or add a separate pass after the main loop that handles session-scope runs.
3. For a session-scope orphan: the run has `RunStarted` and `SessionStarted` but no `RunCompleted`. Append a `RunCompleted` event with:
   - Same `run_id`
   - `outcome: "post_hoc_closed"` or `"incomplete"`
   - Timestamp of closure
4. Add or update a unit test in `scripts/test_append_terminal_state_events.py` that:
   - Creates a mock events list with a session-scope run (RunStarted but no RunCompleted)
   - Calls the closure function
   - Asserts a RunCompleted was appended with the correct outcome
5. Do NOT expand scope: no `closed_by` field, no FailureObserved tracking changes, no refactoring of the script structure. The previous attempt (issue #53) failed because it tried to do all of these at once. This task does ONE thing: detect session-scope orphans and close them.
