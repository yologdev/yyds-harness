Title: Fix state trace timeout on large event histories
Files: src/commands_state.rs
Issue: none
Origin: planner

Evidence:
- Day 164 assessment self-tests: `yyds state trace trace-evolve-31374203467-1-163-09-25` TIMED OUT (20s) on 288K events. The trace command walks the full event store linearly and chokes on accumulated history.
- State doctor shows 288,186 events across 169 runs — the event store grows ~1-2K events per session, so the timeout gets worse every day.
- This is a concrete, verified bug found by assessment self-tests (not a dashboard signal or script inference).

Edit Surface:
- src/commands_state.rs

Verifier:
- cargo build && cargo test -- state_trace -- --test-threads=1

Fallback:
- If `yyds state trace <any-run-id>` completes in under 5s on the current 288K event store before implementation begins, mark this task obsolete — the timeout may have been a transient infrastructure issue rather than a code problem.
- If the fix requires changes outside src/commands_state.rs (e.g., state.rs event reading internals), narrow scope to the trace command's walk loop only and file a follow-up issue for the lower-level change.

Objective:
Make `yyds state trace <run-id>` complete in under 5 seconds regardless of total event count. The trace command should not walk all 288K+ events to reconstruct a single run's timeline.

Why this matters:
The `state trace` command is the primary diagnostic tool for inspecting individual run lifecycles. When it times out, operators can't investigate run-specific failures, model-call gaps, or tool errors. Every session adds ~1-2K events, so the timeout only gets worse. A working trace command is essential for debugging the lifecycle gaps and tool failures that show up in trajectory pressure.

Success Criteria:
- `yyds state trace <run-id>` for any run-id in the 288K event store completes in under 5 seconds.
- The trace output is semantically identical to the current output (same events, same ordering, same formatting) — only the scanning strategy changes.
- Existing tests in src/commands_state.rs pass (including trace_report_reconstructs_run_timeline_with_counts).
- The fix does not regress `state tail`, `state doctor`, or any other state command.

Verification:
- cargo build
- cargo test -- state_trace -- --test-threads=1
- time ./target/debug/yyds state trace <a-recent-run-id>  # must finish under 5s
- ./target/debug/yyds state doctor  # must still pass all checks

Expected Evidence:
- Task lineage shows a src/commands_state.rs change that passed strict verification.
- `state trace` no longer appears in assessment self-test failures.
- Trajectory tool-failure metrics do not regress (no new bash_tool_error from timeout).

Implementation Notes:
- The trace function is in src/commands_state.rs. It currently walks the full event list (likely via `read_events_bounded` or direct iteration over all events) to find events matching a trace_id or run_id.
- The fix strategy: instead of walking all events, use a bounded scan. Options:
  a) Scan backwards from the end of the event store (runs are recent, trace events are near the end).
  b) If the run_id or trace_id can be looked up via a cheaper path (e.g., the SQLite projection in state.rs has indexed run data), use that first to narrow the scan range.
  c) Add an early-exit once the trace is fully reconstructed (a trace is a connected subgraph; once all nodes and edges are collected, stop scanning).
- The simplest fix: scan events in reverse (from newest to oldest) and stop when the trace is complete or when events are older than the trace's start time. This is bounded by trace depth, not total event count.
- Keep the change under 50 lines. No new dependencies, no new modules.
- Do NOT change the event store format or the state recording path in state.rs.
- The trace output must remain identical — same events, same ordering (reconstruct forward order after reverse scan), same formatting.
