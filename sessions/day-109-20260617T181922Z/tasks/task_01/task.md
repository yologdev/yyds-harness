Title: Add `state reconcile-events` diagnostic for orphaned tool/model calls
Files: src/commands_state.rs, src/state.rs
Issue: none
Origin: planner

Evidence:
- Trajectory graph pressure row "Reconcile state-only tool failures (state_only_failed_tool_count=10)": State events contained failed tool actions without matching transcript. Before reconciling state vs transcript, first reconcile state internally — find ToolCallStarted events with no ToolCallCompleted and ModelCallStarted events with no ModelCallCompleted.
- Assessment `state summary` counts: "Tool calls: 6,798 started vs 6,810 completed (+12 completions without starts)" and "Model calls: 283 started vs 277 completed (−6 completions)". These discrepancies are measurable and actionable.
- Assessment `state crashes` already detects orphaned runs; a similar orphan-detection pattern for tool/model calls would close a known evidence gap.
- Assessment self-test: `state why last-failure`, `state failures tools`, `state crashes`, and `state summary` all work. The pattern for adding a new `state` subcommand is well established.

Edit Surface:
- src/commands_state.rs: add "reconcile" => "reconcile-events" dispatch in handle_state_subcommand, implement handle_reconcile_events function that reads events, cross-references ToolCallStarted/ToolCallCompleted by tool_call_id and ModelCallStarted/ModelCallCompleted by model_call_id, reports orphans.
- src/state.rs: may add a small helper for extracting tool_call_id/model_call_id from payload (if one doesn't already exist) or reuse existing payload_str helper.

Verifier:
- cargo test -- state reconcile
- cargo check

Fallback:
- If the tool_call_id or model_call_id fields are not consistently populated in the current event stream (check with `yyds state tail --limit 100 | grep -c 'tool_call='`), write a note explaining the data gap instead of shipping a broken command.
- If an existing function already does this reconciliation, mark the task obsolete with proof.

Objective:
Add a `yyds state reconcile-events` diagnostic command that cross-references ToolCallStarted ↔ ToolCallCompleted and ModelCallStarted ↔ ModelCallCompleted events by their IDs and reports orphans (started without completion, completed without start).

Why this matters:
The trajectory flags 10 state-only tool failures — tool failures recorded in state events but absent from transcript logs. Before debugging that state-vs-transcript gap, the harness needs to trust its own internal event integrity. If state has orphaned tool calls (starts without completions, completions without starts), then the failure counts in `state failures tools` and `state summary` are unreliable. This diagnostic makes the evidence trustworthy first, then enables accurate transcript reconciliation later.

Success Criteria:
- `yyds state reconcile-events` (or `yyds state reconcile events`) produces a report showing:
  - Tool calls: N started, M completed, X orphans (started without completion), Y orphans (completed without start)
  - Model calls: N started, M completed, X orphans (started without completion), Y orphans (completed without start)
  - Optionally: the specific orphan event IDs and tool_call_ids/model_call_ids
- `--limit N` flag controls how many orphan details to show
- Empty output or "all events paired" when no orphans found
- Follows the same pattern as `state failures tools` (read events, build report, print)

Verification:
- cargo test -- state reconcile
- cargo build
- yyds state reconcile-events --limit 5 (in a session with events)
- yyds state reconcile-events (on a fresh/empty state directory — graceful)

Expected Evidence:
- Future `state summary` can cite reconciled counts
- Dashboard/trajectory can distinguish "orphaned in state" from "present in state but missing in transcript"
- Task lineage artifact shows the new subcommand wired and testable

Implementation Notes:
- Hook into `handle_state_subcommand` in `src/commands_state.rs` near line 27. The dispatch currently handles "tail", "why", "graph", "failures", "summary", "crashes", "init". Add "reconcile" → handle_reconcile_events.
- The "reconcile" subcommand can also accept sub-subcommands ("events" is the default/first one). Use "reconcile" as the dispatch key and "events" as the default action, leaving room for future "reconcile transcript" or "reconcile model" variants.
- Use the existing `read_events_lenient` function (used by handle_failures, handle_tool_failures) to load event data.
- Cross-reference algorithm: iterate events, collect ToolCallStarted by tool_call_id into a HashMap, then remove entries when ToolCallCompleted is found for the same tool_call_id. Remaining entries are orphans. Do the same for ModelCallStarted/ModelCallCompleted by model_call_id.
- Use `payload_str(&event.payload, "tool_call_id")` and `payload_str(&event.payload, "model_call_id")` for ID extraction (these helpers exist in `src/state.rs`).
- Keep the change under 150 lines. This is a focused diagnostic, not a full reconciliation engine.
