Title: Improve cold-start state why last-failure diagnostics
Files: src/commands_state.rs
Issue: none
Origin: planner (refined from harness-seed)

Objective:
Make `yyds state why last-failure` report actionable diagnostic paths when no completed failed sessions exist — surfacing incomplete runs (RunStarted without RunCompleted), startup errors, and suggesting which state subcommands to try next.

Why this matters:
The state CLI is the first diagnostic tool a developer uses when something goes wrong. Currently `state why last-failure` returns "no failures recorded" when all sessions succeeded — accurate but unhelpful. The harness frequently encounters cold-start and preflight scenarios (no-api-key, empty-input, timeout) that produce incomplete runs without reaching the failure threshold. Making `why last-failure` aware of incomplete runs turns a dead-end response into a diagnostic breadcrumb trail.

Success Criteria:
- When no completed failed runs exist but incomplete runs (RunStarted without RunCompleted) are present, `state why last-failure` reports them with their run IDs and a hint to run `state trace <id>` or `state crashes`.
- When no state events exist at all, the existing cold-start message is preserved (already good).
- The `--summary` flag still works and includes incomplete-run counts.
- Existing behavior for completed failed sessions is unchanged.

Verification:
- cargo test commands_state why -- --test-threads=1
- cargo build && cargo check
- Manual: run `yyds state why last-failure` in a fresh state (or with only successful sessions) and verify it surfaces incomplete runs if any exist.

Expected Evidence:
- Future state tail output shows improved `state why` responses in cold-start scenarios.
- The `state why` output now distinguishes three cases: (a) failed session found → report it, (b) no failures but incomplete runs → report them with diagnostic next-steps, (c) no state at all → cold-start message.

Implementation Notes:
- The `handle_why` function at `src/commands_state.rs:836` calls `build_why_report(&events, id)`. When this returns an error ("no failures recorded"), the implementation should scan for incomplete runs before returning.
- Incomplete runs are detected by finding `RunStarted` events whose run_id has no matching `RunCompleted` event in the scanned event window.
- Use `read_tail_events` (already imported) for the scan. The `--limit` flag controls how many events are scanned; respect it.
- Do NOT change the return type of `build_why_report` — add the incomplete-run scan as a fallback in `handle_why` after `build_why_report` returns an error.
- Keep the change scoped to `handle_why` and possibly a small helper function. Do not touch `build_why_report` internals.
- The output format should be consistent with existing `state why` styling: use `{YELLOW}`/`{DIM}`/`{RESET}` from `crate::format`.
- Example output when no failures but incomplete runs:
  ```
  No failed sessions found in recent history.
  However, 2 incomplete runs were detected (started but not completed):
    run-abc123 — started 10m ago, no RunCompleted event
    run-def456 — started 25m ago, no RunCompleted event
  Run: yyds state trace <run-id> for details, or yyds state crashes for crash analysis.
  ```
