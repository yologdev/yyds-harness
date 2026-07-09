Title: Teach append_terminal_state_events.py to recognize SessionStarted as a lifecycle start event
Files: scripts/append_terminal_state_events.py
Issue: none
Origin: planner (refined from harness seed + trajectory evidence)

Evidence:
- Graph-derived next-task pressure #1: "Close yyds state and model lifecycle gaps" with `state_run_incomplete_count=1`, lifecycle causes include `state_incomplete/open_after_SessionStarted=1` (from trajectory, Day 131)
- `lifecycle_for_scope()` at line 92 only checks for `RunStarted` and `RunCompleted` — `SessionStarted` events pass through unrecognized, leaving SessionStarted-only runs permanently open
- `find_stale_orphaned_runs()` at line 175 has the same blind spot: only recognizes `RunStarted`, not `SessionStarted`
- The Rust state layer already emits `SessionStarted` events (src/state.rs line 578 via `record_session_started`, called from src/lib.rs line 1131). These events carry valid run_ids and session metadata. The Python diagnostic layer needs to see them.
- Day 130 (10:20) already fixed input-validation filtering for unmatched completions — this gap is in lifecycle-event recognition, not completion filtering

Edit Surface:
- scripts/append_terminal_state_events.py (two functions: `lifecycle_for_scope` ~line 92 and `find_stale_orphaned_runs` ~line 198)

Verifier:
- python3 -c "import scripts.append_terminal_state_events; print('import ok')"
- python3 -m pytest scripts/ -k "test_append_terminal" 2>/dev/null || python3 -m unittest scripts.test_append_terminal_state_events 2>/dev/null || echo "no dedicated test file; verify with: python3 scripts/append_terminal_state_events.py --help"

Fallback:
- If `SessionStarted` events in the live state file already have matching `RunCompleted` events (meaning the gap was closed earlier or never existed in the current state), verify the code change is correct via unit tests on synthetic events and mark the task verified. If the state file has zero `SessionStarted` events, the change is still correct — it prevents future gaps.

Objective:
Close the `open_after_SessionStarted=1` lifecycle gap by teaching the Python diagnostic layer to recognize `SessionStarted` as a lifecycle start event (alongside `RunStarted`), so runs that emit SessionStarted without RunCompleted get detected and retroactively closed.

Why this matters:
The #1 graph-derived pressure item is lifecycle gaps. The Rust state layer (src/state.rs `close_orphaned_run_if_needed`) already handles RunStarted-detected orphans, but the Python diagnostic layer (`append_terminal_state_events.py`) is the harness script that retroactively closes orphaned runs during evolve.sh. If it can't see SessionStarted events, it can't close SessionStarted orphans. This is a Python fix that avoids the complexity that caused the Day 130 Rust-level #83 attempt to fail (the implementation agent got lost tracing event ordering through multiple Rust files). The Python fix is simpler: two functions, same event type addition, same logic.

Success Criteria:
- `lifecycle_for_scope()` treats `SessionStarted` identically to `RunStarted`: adds to `run_started` (or `session_run_started` for session runs), increments `lifecycle_start_count`
- `find_stale_orphaned_runs()` treats `SessionStarted` identically to `RunStarted`: adds to `run_started` (or `session_run_started` for session runs)
- Existing behavior for RunStarted/RunCompleted/ModelCallStarted/ModelCallCompleted is unchanged
- After fix, the next trajectory snapshot shows `open_after_SessionStarted` count dropping to 0 (or the count being replaced by a real completed classification)

Verification:
- python3 -c "from scripts.append_terminal_state_events import lifecycle_for_scope, find_stale_orphaned_runs; print('imports ok')"
- Construct a minimal synthetic event list with SessionStarted but no RunCompleted and verify both functions detect the orphan
- Existing tests must still pass (if a test file exists)

Expected Evidence:
- Future structured state snapshots show `state_run_incomplete_count` dropping or `open_after_SessionStarted` disappearing
- "Close yyds state and model lifecycle gaps" drops off graph-derived pressure in subsequent trajectory reports

Implementation Notes:
The change is two locations in `scripts/append_terminal_state_events.py`:

1. In `lifecycle_for_scope()` (around line 92): Add `"SessionStarted"` to the condition on line 92 alongside `"RunStarted"` and `"RunCompleted"`. The `SessionStarted` event has the same shape as `RunStarted` — it carries a run_id in its payload and can be classified as session/non-session via `is_session_run()`. Treat it exactly like `RunStarted` in all branches.

2. In `find_stale_orphaned_runs()` (around line 198): Same change — add `"SessionStarted"` alongside `"RunStarted"` in the condition on line 198. The function scans the entire event file for orphaned runs; SessionStarted should be recognized as a lifecycle start just like RunStarted.

The SessionStarted event carries a `run_id` field in its payload (same as RunStarted), so `run_id(event, data)` will extract it correctly. No other changes needed — the existing logic for RunCompleted detection, model-call gating, and session-run tracking all work the same way regardless of whether the start event is RunStarted or SessionStarted.

Do NOT modify `src/state.rs` — this is a Python-only fix. Do NOT add new event types or change the event schema. This is purely recognition: SessionStarted already exists in the event stream; the scripts just need to see it.
