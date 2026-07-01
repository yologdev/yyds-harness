Title: Make append_terminal_state_events.py robust against evaluator-timeout orphaned runs
Files: scripts/append_terminal_state_events.py
Issue: none
Origin: planner (refined from harness-seed)

Evidence:
- Assessment §Structured State Snapshot: state_run_incomplete_count=1, cause "state_incomplete/open_after_SessionStarted"
- Day 122 tasks #51 and #52 were reverted because "Evaluator timed out without a verifier verdict" — the evaluator process exited without closing the run lifecycle
- scripts/append_terminal_state_events.py is the post-hoc lifecycle closer that should detect open runs and append RunCompleted events
- The current logic may not cover the evaluator-timeout path: the session's RunStarted exists but no RunCompleted was emitted because the evaluator process died

Edit Surface:
- scripts/append_terminal_state_events.py

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events
- python3 scripts/append_terminal_state_events.py --dry-run (or equivalent self-check)

Fallback:
- If no concrete lifecycle gap exists in the current code (the script already handles this case), write an obsolete note with the actual state and skip edits.
- If the evaluator timeout produces a different event shape than expected, document the gap and close only what's reachable.

Objective:
Ensure runs orphaned by evaluator timeout get proper RunCompleted terminal events during post-hoc lifecycle closure, so state_run_incomplete_count accurately reflects only truly-open runs.

Why this matters:
The evaluator timeout in Day 122 left runs permanently open in the state graph. This inflates state_run_incomplete_count, which feeds into trajectory pressure and can cause the planner to chase phantom lifecycle gaps instead of real harness problems. Fixing the post-hoc closer makes the diagnostic gnome trustworthy.

Success Criteria:
- append_terminal_state_events.py detects runs that have a RunStarted but no RunCompleted and appends a RunCompleted with appropriate outcome (e.g., "evaluator_timeout" or "incomplete")
- The script does not double-close runs that already have a terminal event
- Existing tests continue to pass
- A dry-run on current state shows the orphaned Day 122 run being detected and classified

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events
- python3 scripts/append_terminal_state_events.py --help (confirms the script parses)

Expected Evidence:
- Future structured state snapshots show state_run_incomplete_count = 0 for sessions where the only incomplete runs were evaluator-timeout orphans
- The trajectory extractor no longer flags lifecycle gaps that are actually evaluator-timeout artifacts

Implementation:
1. Read `scripts/append_terminal_state_events.py` to understand the current lifecycle closure logic — how it detects open runs and what events it appends.
2. Identify the evaluator-timeout case: a run that has SessionStarted or RunStarted but no RunCompleted or FailureObserved. This is distinct from a crash (which may have FailureObserved) or a normal completion.
3. Add logic to detect such orphaned runs and append a RunCompleted event with an outcome field indicating the run was closed post-hoc (e.g., `outcome: "evaluator_timeout"` or `closed_by: "post_hoc"`).
4. Ensure the fix distinguishes between: (a) runs still in-progress (current session — do NOT close), (b) runs that already have a terminal event (skip), (c) runs orphaned by process death (close with explanatory outcome).
5. Add or update unit tests in the test file to cover the evaluator-timeout orphan case.
