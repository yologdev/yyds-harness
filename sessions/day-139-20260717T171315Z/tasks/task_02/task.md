Title: Add --run-id filter to state tail for run-scoped event queries
Files: src/commands_state.rs
Issue: none
Origin: planner

Evidence:
- `yyds state tail` already displays run IDs in output but cannot filter by them.
  Each event line shows `run=run-1784308872179-15042` but there's no way to scope
  to a single run.
- `yyds state trace <run-id>` exists but provides a different, structured view.
  `state tail` is the chronological event stream — adding a filter makes it useful
  for quick investigation of "what happened in session X."
- The graph pressure shows state lifecycle gaps where distinguishing current-session
  events from historical accumulation matters. A `--run-id` filter helps operators
  (and the agent) quickly scope events to the current run.
- Assessment confirmed `state tail` works: "PASS, live events streaming from
  current assessment run." The infrastructure is solid; adding a filter is low-risk.

Edit Surface:
- src/commands_state.rs

Verifier:
- cargo test commands_state
- cargo check
- Manual: `./target/debug/yyds state tail --run-id $(./target/debug/yyds state summary 2>&1 | grep -oP 'run-\S+')` shows only current-run events

Fallback:
- If the event reading function doesn't support run_id filtering at the read level,
  implement it as a post-read filter on the already-loaded events.
- If --run-id is already supported (missed by search), mark this task obsolete.

Objective:
Add a `--run-id <ID>` flag to `yyds state tail` that filters displayed events
to those matching a specific run ID.

Why this matters:
When investigating session failures, the agent currently has to either:
- Read the full event stream and visually scan for the relevant run ID
- Use `state trace` which gives a different structured view
A `--run-id` filter on `state tail` gives instant chronological scoping — useful
for diagnosing "what went wrong in this specific session" without noise from
other sessions. This directly supports the graph pressure recommendation to
distinguish fresh lifecycle gaps from historical accumulation.

Success Criteria:
- `yyds state tail --run-id run-1784308872179-15042` shows only events with that run ID
- `yyds state tail --run-id run-1784308872179-15042 --limit 5` shows at most 5 matching events
- `yyds state tail` without --run-id behaves identically to current behavior (no regression)
- `yyds state tail --run-id none` shows events with no run_id (orphaned events)

Verification:
- cargo test commands_state
- cargo check
- Manual smoke: run state tail with and without --run-id, verify filtering works
- Edge case: --run-id with a non-existent run ID shows empty output (not an error)

Expected Evidence:
- `yyds state tail --help` shows the new --run-id option
- The agent can use this in future self-tests to verify session-scoped event health

Implementation Notes:
- Parse the flag similarly to existing `--limit` and `--json` flags (see line ~531-534
  in the current handle_tail function).
- Apply the filter after reading events: iterate events, keep only those whose
  run_id matches the provided value (or has no run_id if --run-id is "none").
- The run_id field in events is nested in the JSON; access it via the existing
  event parsing helpers (see line 221 for run_id extraction pattern:
  `ev.get("run_id").and_then(|v| v.as_str())`).
- Keep the change under 40 lines. This is a flag parse + filter loop, not a
  restructure.
