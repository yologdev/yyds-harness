Title: Close orphaned state runs left open after FailureObserved
Files: src/state.rs
Issue: none
Origin: planner

Evidence:
- Trajectory: `deepseek_model_call_abnormal_completed_count=1` with `state_unmatched/open_after_FailureObserved=3` — three runs have FailureObserved events but no matching RunCompleted.
- Day 142 Task 2 added ModelCallStarted/Completed pairing guard in prompt.rs, but that only prevents NEW orphans. The 3 existing orphans are pre-existing and won't self-heal.
- `yyds state doctor` reports clean currently (it may not detect this specific lifecycle gap type), but the trajectory extractor sees the imbalance across sessions.
- RunCompleted is critical — without it, state projections, dashboard session counts, and gnome metrics can't correctly close runs.
- This is graph pressure row #2: "Close yyds state and model lifecycle gaps."

Edit Surface:
- src/state.rs

Verifier:
- cargo test state -- --test-threads=1
- cargo build

Fallback:
- If the FailureObserved events all have matching RunCompleted events (race was already fixed by Day 142 Task 2 and trajectory data was stale), write `session_plan/task_01_obsolete.md` explaining the exact check that confirmed no orphans remain.
- If adding the repair to state.rs requires parsing events from SQLite (too complex for 20min), scope down to adding a `yyds state repair-orphaned-runs` command that calls the existing `append_terminal_state_events.py` script and verifies the result.

Objective:
Add a state repair function in `src/state.rs` that detects and closes orphaned runs — runs that have a FailureObserved event but no subsequent RunCompleted event within the same run_id. This prevents stale lifecycle gaps from distorting dashboard metrics and trajectory analysis.

Why this matters:
The trajectory shows `open_after_FailureObserved=3` — three runs that crashed or were interrupted without properly closing. Day 142's ModelCallStarted/Completed guard prevents new orphans, but existing ones remain. Each orphaned run leaves stale state in the dashboard and can cause session-count skew. The state repair should be callable automatically at session start (in evolve.sh) and also from `yyds state doctor` or a new `yyds state repair` command path.

Success Criteria:
- A function `close_orphaned_failure_runs()` (or similar name) in src/state.rs that:
  - Scans the event store for runs with FailureObserved but no RunCompleted on the same run_id.
  - For each orphaned run, emits a RunCompleted event with `status: "error"` and a note indicating it was auto-closed by repair.
  - Returns the count of runs closed.
- The function is idempotent — running it twice produces no new events on the second run.
- `cargo test` passes (existing state tests + new test).

Verification:
- cargo test state -- --test-threads=1
- cargo build
- After build, run `yyds state doctor` to confirm no regressions.

Expected Evidence:
- After evolve.sh calls the repair function (or after manual invocation), trajectory snapshots show `open_after_FailureObserved=0`.
- Dashboard session counts and run lifecycle reports no longer show unmatched FailureObserved events.

Implementation Notes:
- Use the existing `StateRecorder` / event-reading infrastructure. The current state store uses a JSONL file at `state/events.jsonl` (or the configured path).
- The function should iterate through events, track run_id → has_FailureObserved mapping, and check for RunCompleted.
- At the end, for each run_id with FailureObserved but no RunCompleted, call `record(EventType::RunCompleted, Actor::Harness, payload)` with status "error" and a note like `{"reason": "auto-closed: orphaned after FailureObserved"}`.
- Write a unit test that:
  1. Records a FailureObserved event for run_id "test-orphan"
  2. Calls the repair function
  3. Asserts that a RunCompleted event was written for "test-orphan"
  4. Calls repair again, asserts no duplicate events
- Keep the change scoped to src/state.rs. Do not modify scripts/ unless needed for verification.
- The existing `install_panic_hook` already emits RunCompleted in the panic path (check around line 150-200 of state.rs) — the new function covers runs that were interrupted externally (GH Actions cancellation, SIGTERM) where the panic hook never fired.
- Use `global_state_recorder()` or equivalent to access the current recorder. Check existing patterns in state.rs for how events are recorded and read back.
