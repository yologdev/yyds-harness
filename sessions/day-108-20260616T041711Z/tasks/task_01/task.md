Title: Show incomplete run IDs and lifecycle detail in `state why last-failure` cold-start diagnostics
Files: src/commands_state.rs
Issue: none
Origin: planner (refined from harness-seed)

Objective:
When `state why last-failure` finds no completed failures, show the specific incomplete RunStarted event IDs and timestamps so the user can investigate the active or orphaned session directly rather than getting generic guidance.

Why this matters:
The current `build_why_report` (line 2635) already provides useful cold-start guidance: it distinguishes "no sessions yet" from "session in progress" from "N successful sessions." But it doesn't give the user a concrete run ID or timestamp to investigate. When a run started but never completed (orphaned run), the user currently gets "A session is currently in progress" with no way to identify which run. Adding the run ID and timestamp turns this from generic advice into actionable diagnostic handles. This closes the remaining gap from the harness-seed task without redoing what's already working.

Success Criteria:
- `yyds state why last-failure` with an active incomplete run shows the `run_id` and start time of the incomplete `RunStarted` event
- Existing behavior for completed failed sessions, empty state, and successful sessions is unchanged
- Output still includes the existing `state crashes`, `state tail`, and `state why last-crash` hints

Verification:
- cargo check
- cargo test --bin yyds -- --test-threads=1 commands_state
- Manual: `./target/debug/yyds state why last-failure` (during an active session, should show run ID + timestamp)

Expected Evidence:
- Future assessment self-tests can cite the specific run ID from cold-start diagnostics
- State/dashboard blockers become easier to trace to run/session IDs
- No regression in existing failure diagnostics

Implementation Notes:
- The work is entirely in `src/commands_state.rs` in `build_why_report` (line 2635)
- In the `run_completed_count == 0 && run_started` branch (line 2668), collect `RunStarted` events, extract their `run_id` and timestamp, and include them in the output
- Keep the change minimal — do not refactor other parts of the file
- Use existing helper functions: `event_string(e, "run_id")`, `event_timestamp_ms(e)`, `format_timestamp_ms`
- Filter to only incomplete runs: RunStarted events whose run_id doesn't appear in any RunCompleted event
- Limit to at most 5 incomplete runs to avoid overwhelming output
