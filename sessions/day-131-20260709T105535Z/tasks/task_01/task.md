Title: Teach append_terminal_state_events.py to recognize SessionStarted as a lifecycle start event
Files: scripts/append_terminal_state_events.py
Issue: #84
Origin: planner (refined from harness seed + trajectory evidence)

Evidence:
- Trajectory structured state: `state_run_incomplete_count=1` with lifecycle cause `open_after_SessionStarted=1` — the #1 structured state pressure on Day 131
- `lifecycle_for_scope()` at line 92 only checks for `"RunStarted"` and `"RunCompleted"` — `SessionStarted` events pass through unrecognized
- `find_stale_orphaned_runs()` at line 198 has the same blind spot: only recognizes `"RunStarted"`, not `"SessionStarted"`
- The Rust state layer emits `SessionStarted` events (src/state.rs, via `record_session_started`). These events carry valid run_ids. The Python diagnostic layer needs to see them.
- Day 131 issue #84 reverted because the task file had no `Files:` entries — the fix was correct, the task file format was wrong

Edit Surface:
- scripts/append_terminal_state_events.py: two functions — `lifecycle_for_scope` (line 92, 94, 99) and `find_stale_orphaned_runs` (line 198, 199, 203)

Verifier:
- python3 -c "from scripts.append_terminal_state_events import lifecycle_for_scope, find_stale_orphaned_runs; print('imports ok')"

Fallback:
- If `SessionStarted` events in the live state file already have matching `RunCompleted` events (gap already closed), verify correctness via synthetic test events and mark verified. If zero `SessionStarted` events exist in the state file, the change is still correct — it prevents future gaps.

Objective:
Close the `open_after_SessionStarted=1` lifecycle gap by teaching the Python diagnostic layer to recognize `SessionStarted` as a lifecycle start event (alongside `RunStarted`), so runs that emit SessionStarted without RunCompleted get detected and retroactively closed.

Why this matters:
The #1 graph-derived structured state pressure is `state_run_incomplete_count=1` caused by `open_after_SessionStarted=1`. The Rust state layer already handles RunStarted-detected orphans, but the Python diagnostic layer (append_terminal_state_events.py) is the harness script that retroactively closes orphaned runs during evolve.sh. If it can't see SessionStarted events, it can't close SessionStarted orphans.

Success Criteria:
- `lifecycle_for_scope()` treats `SessionStarted` identically to `RunStarted` in both the session-run branch (line 92/94) and the regular-run branch (line 99)
- `find_stale_orphaned_runs()` treats `SessionStarted` identically to `RunStarted` in both the session-run branch (line 198/199) and the regular-run branch (line 203)
- SessionStarted events contribute to `lifecycle_start_count` in both functions
- Existing behavior for RunStarted/RunCompleted/ModelCallStarted/ModelCallCompleted is unchanged

Verification:
- python3 -c "from scripts.append_terminal_state_events import lifecycle_for_scope, find_stale_orphaned_runs; print('imports ok')"
- Construct a minimal synthetic event list with SessionStarted but no RunCompleted and verify both functions detect the orphan

Expected Evidence:
- Future structured state snapshots show `open_after_SessionStarted` count dropping to 0
- `state_run_incomplete_count` decreases in subsequent trajectory reports

Implementation Notes:
The change is four conditions in two functions:

1. In `lifecycle_for_scope()`:
   - Line 92: Add `"SessionStarted"` to the set: `{"RunStarted", "SessionStarted", "RunCompleted"}`
   - Line 94: Add `or kind == "SessionStarted"` to `if kind == "RunStarted":`
   - Line 99: Add `or kind == "SessionStarted"` to `if kind == "RunStarted":`

2. In `find_stale_orphaned_runs()`:
   - Line 198: Add `"SessionStarted"` to the set: `{"RunStarted", "SessionStarted", "RunCompleted"}`
   - Line 199: Add `or kind == "SessionStarted"` to `if kind == "RunStarted":`
   - Line 203: Add `or kind == "SessionStarted"` to `elif kind == "RunStarted":`

SessionStarted events carry a `run_id` field in their payload (same shape as RunStarted), so `run_id(event, data)` will extract it correctly. No other changes needed — the existing logic for RunCompleted detection, model-call gating, and session-run tracking all work the same way regardless of whether the start event is RunStarted or SessionStarted.

Do NOT modify `src/state.rs` — this is a Python-only fix. Do NOT add new event types or change the event schema. This is purely recognition: SessionStarted already exists in the event stream; the scripts just need to see it.
