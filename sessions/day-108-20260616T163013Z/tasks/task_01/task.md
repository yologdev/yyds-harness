Title: Add state failures tools subcommand for tool-failure reconciliation
Files: src/commands_state.rs, src/state.rs
Issue: none
Origin: planner

Objective:
Add a `yyds state failures tools [--limit N]` subcommand that filters ToolCallCompleted state
events with error/failure payloads, showing tool name, error type, run context, and timestamps.
This makes state-recorded tool failures visible at the CLI, enabling agents to reconcile the
16 state-only vs 1 transcript-only tool-failure gap identified by graph pressure.

Why this matters:
The trajectory's graph-derived pressure ranks "Reconcile state-only tool failures
(state_only_failed_tool_count=16)" as the #1 actionable signal. Current `state failures --recent`
shows a generic failure report but doesn't break down failures by tool or distinguish tool-level
failures from run-level failures. The 16:1 state-to-transcript discrepancy means either state is
over-recording tool failures or transcripts are under-reporting them — without a dedicated
diagnostic view, agents can't tell which.

An addition to `state` diagnostic surface directly improves the DeepSeek harness's self-healing
capacity: when agents can see which tool calls failed and why, they can add prompt/tool guards
for the dominant failure class. This is the kind of observability investment that compounds
across sessions.

Success Criteria:
- `yyds state failures tools` shows ToolCallCompleted events with error/failure payloads
- Output includes tool name, error/failure type, run_id, and timestamp
- Default limit is 10, overridable with --limit N
- Falls back gracefully when no tool-failure events exist: prints "no tool failures found"
- Does not break existing `state failures --recent` behavior
- The new subcommand path is `state failures tools` (not `state failures --tools`)

Verification:
- cargo check
- cargo test --bin yyds -- --test-threads=1
- cargo fmt --check

Expected Evidence:
- Future `state failures tools` output shows tool-level failure detail that agents can act on
- Graph-derived tool-failure pressure charts converge (state-only count becomes reconcilable)
- Dashboard tool-failure reconciliation metrics move toward 0 discrepancy

Implementation Notes:
- Extend the `handle_failures` function in `src/commands_state.rs` to accept `tools` as a
  sub-subcommand: when `args.first()` is `"tools"`, dispatch to a new handler.
- The new handler reads events via `read_events_lenient`, filters for `ToolCallCompleted`
  events where the payload contains error/failure indicators (look for `"error":` or
  `"status": "failed"` or `"success": false` in the JSON payload), and formats them.
- Each row should show: timestamp (ISO), tool name, error/failure summary (first 80 chars of
  error field), and run_id.
- If `src/state.rs` needs a helper for extracting tool-call payload fields, add it there.
  Otherwise keep the implementation local to `commands_state.rs`.
- Keep the change small — this is a diagnostic addition, not a recording overhaul.
- Use existing patterns: `read_events_lenient`, `flag_value`, color constants (BOLD, YELLOW,
  RED, GREEN, RESET) from the existing code.
