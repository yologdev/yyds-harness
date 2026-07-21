Title: Close all orphaned FailureObserved runs, not just the most recent dangling one
Files: src/state.rs
Issue: #129
Origin: planner

Evidence:
- Trajectory graph pressure row #2: `open_after_FailureObserved=3` — three runs have FailureObserved events but no matching RunCompleted.
- Assessment (Day 143): "Day 142 Task 2 prevents NEW orphans at the ModelCall level, but existing orphans persist."
- Assessment explicitly recommends: "retry #129 with smaller scope — add the repair function with tests, keep eval simple."
- Existing `close_orphaned_run_if_needed` (line 323) only handles the *single most-recent* dangling RunStarted/SessionStarted. It stops at the first lifecycle event found when scanning backward. If a healthy run with RunCompleted intervenes after an orphaned FailureObserved run, the orphan is missed.
- Day 142 Task 2 added ModelCallStarted/Completed pairing guard in prompt.rs — prevents new model-level orphans. But the 3 existing run-level orphans remain.

Edit Surface:
- src/state.rs

Verifier:
- cargo test state -- --test-threads=1
- cargo build

Fallback:
- If `close_orphaned_run_if_needed` already handles all FailureObserved runs (not just most-recent) after closer inspection, write `session_plan/task_01_obsolete.md` with the exact line numbers proving coverage.
- If extending the existing function requires more than ~60 lines of Rust, scope down to adding a separate `close_all_orphaned_failure_runs()` that is called immediately after the existing check in `init_global`, and add one focused test.

Objective:
Extend `close_orphaned_run_if_needed` (or add a companion function) so that ALL runs with a FailureObserved event but no matching RunCompleted are auto-closed with a retroactive RunCompleted at session startup — not just the single most-recent dangling run.

Why this matters:
The trajectory shows `open_after_FailureObserved=3` — runs that crashed or were interrupted without closing. Each orphan skews dashboard session counts, run lifecycle reports, and gnome metrics. Day 142's guard prevents new orphans, but existing ones persist and accumulate. The state doctor doesn't currently flag these explicitly, creating silent metric distortion.

Success Criteria:
- After `init_global` runs (at session startup), ALL runs with FailureObserved but no RunCompleted in the events file have a retroactive RunCompleted emitted.
- The function is idempotent: running it twice produces no duplicate RunCompleted events.
- The existing most-recent-dangling-run behavior is preserved and not broken.
- `cargo test` passes including the existing `init_global_closes_orphaned_run_from_previous_session` test.

Verification:
- cargo test state -- --test-threads=1
- cargo build

Expected Evidence:
- Next trajectory snapshot shows `open_after_FailureObserved=0`.
- State doctor / `yyds state why` no longer shows unmatched FailureObserved events.
- Dashboard run lifecycle counts are correct.

Implementation Notes:
- The fix is in `close_orphaned_run_if_needed` at line 323 in `src/state.rs`.
- Current behavior: scans backward from end of events file, stops at first lifecycle event (RunStarted, SessionStarted, or RunCompleted). If that event is RunStarted/SessionStarted without a matching RunCompleted, closes it. If it's RunCompleted, returns Ok.
- Gap: if run-A had FailureObserved but no RunCompleted, and run-B later ran successfully with RunCompleted, the backward scan finds RunCompleted for run-B and returns Ok — missing run-A entirely.
- Fix: after the backward scan for the most-recent run, add a forward pass that collects all run_ids with FailureObserved events, then checks each against RunCompleted. For any run_id with FailureObserved but no RunCompleted, emit a retroactive RunCompleted.
- Alternatively: restructure to do one full forward pass that tracks run_id → has_failure + has_completed, then closes any with failure but no completed. This is cleaner but changes more of the existing function.
- Preferred approach (minimal diff): after the existing backward scan block (line 386), add a second pass:
  ```rust
  // Second pass: find all runs with FailureObserved but no RunCompleted
  // The backward scan above only catches the most recent dangling run.
  let mut failure_runs: std::collections::HashSet<String> = std::collections::HashSet::new();
  let mut completed_runs: std::collections::HashSet<String> = std::collections::HashSet::new();
  for event in &events {
      let et = event.get("event_type").and_then(|v| v.as_str()).unwrap_or("");
      let rid = event.get("run_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
      if rid.is_empty() { continue; }
      if et == "FailureObserved" { failure_runs.insert(rid); }
      if et == "RunCompleted" { completed_runs.insert(rid); }
  }
  for rid in &failure_runs {
      if completed_runs.contains(rid) || orphan_run_id.as_deref() == Some(rid) { continue; }
      // Emit retroactive RunCompleted
      let event = StateEvent {
          event_id: format!("evt-orphan-failure-{}", now_ms()),
          event_type: EventType::RunCompleted,
          schema_version: STATE_SQLITE_SCHEMA_VERSION,
          timestamp_ms: now_ms(),
          actor: Actor::Harness,
          run_id: Some(rid.clone()),
          session_id: None,
          trace_id: format!("trace-orphan-failure-{}", rid),
          parent_event_ids: Vec::new(),
          payload: run_completed_payload(
              "error",
              Some("auto-closed: orphaned after FailureObserved"),
              None,
          ),
      };
      append_event_with_projection(events_path, store_path, &event)?;
  }
  ```
- The `HashSet` approach is O(n) and safe for the event volume we have (~200K events).
- The `orphan_run_id` dedup check ensures the first pass and second pass don't both close the same run.
- Add a test: create events with FailureObserved for run-X, then RunStarted + RunCompleted for run-Y. Call `close_orphaned_run_if_needed`. Assert run-X gets RunCompleted, run-Y does not get a duplicate.
- Keep the total change under 60 lines. Do not refactor the entire function — add the second pass after the existing block.
