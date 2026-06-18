Title: Add per-session tool failure grouping to `state failures --recent`
Files: src/commands_state.rs
Issue: none
Origin: planner

Evidence:
- Trajectory graph pressure #1: `transcript_only_failed_tool_count=2` — transcript logs contain tool failures absent from state events
- Trajectory graph pressure #2: `state_only_failed_tool_count=13` — state events contain tool failures unmatched in transcript records
- These are computed by `scripts/build_evolution_dashboard.py` line 2666-2667 using `unique_delta_count()` on state vs transcript tool failure sets
- The `state failures --recent` command (src/commands_state.rs line 831) already reads tool failure events but reports them as a flat list — no session grouping, making it impossible to cross-reference with per-session transcripts
- Without session-level grouping, diagnosing the 15 mismatched events requires manual correlation across flat event lists

Edit Surface:
- src/commands_state.rs — `handle_tool_failures` (line 831) and `build_tool_failures_report` (line 1993)

Verifier:
- cargo test commands_state
- cargo test -- failures

Fallback:
- If `state failures --recent` already groups by session or run ID in current HEAD (unlikely per assessment), close as already-done.

Objective:
Extend `yyds state failures --recent` to group tool failure events by session run ID, showing how many failures occurred in each session. This makes it possible to manually cross-reference state-recorded failures against per-session transcript logs, directly enabling diagnosis of the state/transcript reconciliation gap.

Why this matters:
The #1 trajectory pressure is the state/transcript tool failure mismatch. Before we can fix the root cause (whether it's a recording gap in state or a parsing gap in transcripts), we need to see *which sessions* have the mismatch. Grouping failures by session is the minimum viable diagnostic that turns a flat list of 13+ mismatched events into actionable per-session evidence.

Success Criteria:
- `yyds state failures --recent` groups output by run ID (session), e.g.:
  ```
  run github-actions-27735942108 (2 failures):
    ToolCallFailed: bash command `cargo build` exit=1 ...
    ToolCallFailed: read_file path `src/nonexistent.rs` ...
  ```
- Running without `--recent` still works (shows all failures)
- Existing `--limit` flag still works
- The flat failure detail view is preserved — grouping is additive, not a replacement
- No regression in `cargo test`

Verification:
- cargo build
- cargo test commands_state
- cargo test -- failures
- Manual: `cargo run -- state failures --recent --limit 20` → verify session grouping

Expected Evidence:
- `state failures --recent` output organized by run ID, enabling per-session cross-reference with transcripts
- The session grouping surfaces which sessions have the most tool failures, highlighting candidates for reconciliation investigation
- Future dashboard scoring or `log_feedback.py` can consume per-session failure counts from this command

Implementation Notes:
- The `handle_tool_failures` function at line 831 reads all tool failure events via `read_events_lenient` and passes them to `build_tool_failures_report`.
- `build_tool_failures_report` at line 1993 currently formats failures as a flat list. Extend it to group by a run/session identifier before formatting.
- Look for a `run_id` or `session_id` field in the event payload. If tool failure events don't carry a run ID, use the nearest preceding `SessionStarted` or `RunStarted` event to infer grouping — or check how `state why` finds run context.
- Keep the change minimal: add a `--by-session` flag or make session grouping the default for `--recent`. If making it default, ensure the output remains readable.
- If the event payload doesn't have a direct session/run field, consider grouping by the event's implicit batch (events between RunStarted/RunCompleted). Check how `state summary` or `state lifecycle` does this grouping.
