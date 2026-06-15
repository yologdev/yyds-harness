Title: Fix state event type classification — 9,293 events all typed "unknown"
Files: src/state.rs
Issue: none
Origin: planner (replaces stale harness-seed)

Note: The harness seed task_01.md targeted cold-start `state why` diagnostics, but assessment evidence shows this was already fixed in Day 107 (03:21): `state why last-failure` now produces "clean cold-start message with diagnostic paths." Replaced with a higher-priority evidenced task.

Objective:
Make `yyds state doctor` and `yyds state lifecycle` correctly classify recorded events so runs, failures, and lifecycles are queryable instead of all appearing as "unknown."

Why this matters:
Assessment evidence: 9,293 events recorded in the events file, visible in `state tail` with run IDs and tool names, but `state doctor` reports 0 runs, 0 failures, and all event types as "unknown." `state lifecycle` returns "runs: 0 started, 0 completed, 0 incomplete." This means the SQLite projection layer is not mapping raw events to typed records. The entire state infrastructure — evals, patches, claims, graph pressure, trajectory computation — depends on typed state events. With classification broken, none of these systems produce reliable output.

Success Criteria:
- `yyds state doctor` shows non-zero run count and correct event type distribution (not all "unknown")
- `yyds state lifecycle` shows non-zero started/completed/incomplete runs
- Existing events (the 9,293 already recorded) are classifiable after fix
- No regression: `state tail` and other state subcommands still work
- `cargo test` passes, including any state-related tests

Verification:
- cargo build && cargo test --lib state
- ./target/debug/yyds state doctor (should show typed events, non-zero runs)
- ./target/debug/yyds state lifecycle (should show non-zero runs)
- ./target/debug/yyds state tail --limit 5 (should still work)

Expected Evidence:
- state doctor output shows event type distribution with real types (ToolCallStarted, FileRead, CommandCompleted, etc.) instead of all "unknown"
- state lifecycle shows runs: N started, M completed, K incomplete where N > 0
- state graph and evals can begin producing meaningful query results
- Future assessment self-test: `state doctor` no longer flagged with ⚠️

Implementation Notes:
- The bug is in the event-to-SQLite mapping path in `state.rs`. Events are written to the events file in a format visible in `state tail`, but the SQLite projection migration/rebuild either doesn't run or doesn't recognize the event format.
- Investigate the `normalize_event_json_line` / `compatibility_event_json_line` functions — these convert raw event JSON to the format the SQLite projection expects. If events are stored in a format these functions don't recognize, they'll fall through to "unknown."
- Check the `rebuild_sqlite_projection` / `migrate_sqlite_projection` path — it may not be triggered, or the migration may be failing silently.
- Check whether the `read_compatibility_events` function is returning any events. If it returns empty, the projection has nothing to work with.
- Keep the fix scoped to `state.rs` only. If the root cause spans multiple files, note it and create a follow-up task rather than expanding scope.
- A good diagnostic first step: run `yyds state doctor` with some debug output to see which step in the pipeline drops the events.
