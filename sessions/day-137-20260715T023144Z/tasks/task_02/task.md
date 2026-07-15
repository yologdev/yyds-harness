Title: Expand graph evidence relation filter to include runtime event relations
Files: src/commands_state.rs
Issue: none
Origin: planner

Evidence:
- Assessment self-test: `yyds state graph evidence <event_id>` returns "no graph evidence relations found" for runtime events. This was confirmed by the Phase A1 assessor on Day 137.
- `build_graph_evidence_report` (commands_state.rs:6080) calls `query_graph_timeline` then filters through `graph_evidence_relation` (commands_state.rs:11578). The filter only accepts evaluation/promotion relation types (e.g. "addresses", "evaluated_failure", "explains", "supports") but the SQLite `state_relations` table stores runtime relations like "observed_in", "derived_from", "references_file", "modified_file", "records_model_call", "uses_model", "traced_by" (state.rs:1579-1674). No runtime event can match the evidence filter, so the command always returns empty.
- Graph-derived pressure: "Reconcile state-only tool failures (state_only_failed_tool_count=49)" and "Recover failed tool actions before scoring (tool_error_count=2)" — making the evidence view work would let us query what happened around specific failure events.

Edit Surface:
- src/commands_state.rs (the `graph_evidence_relation` function at line 11578)

Verifier:
- cargo test --lib state -- --test-threads=1
- After build, the implementation agent should check: `cargo run -- state graph evidence <a-recent-event-id> --depth 2` produces non-empty output

Fallback:
- If the SQLite `state_relations` table has zero rows (no graph projection), or the `query_graph_timeline` function returns results but none match even the expanded filter, write an analysis of which relation types exist and whether the projection is being built at all. Do not add fake relations.

Objective:
Make `yyds state graph evidence <event_id>` produce meaningful output for runtime events by adding the common runtime relation types to the `graph_evidence_relation` filter.

Why this matters:
The state graph is yyds's primary observability tool for understanding what happened during and around failures. The timeline view works (it shows all relations), but the evidence view — which should show provenance and impact relations — is a no-op because its filter is too narrow. Fixing this makes "Reconcile state-only tool failures" and "Recover failed tool actions" actionable: we can trace what files were read/modified around a failure, which run it occurred in, and what model calls were involved.

Success Criteria:
- `yyds state graph evidence <event-id> --depth 2` returns at least one relation line for a recent runtime event (FileRead, FileEdited, ToolCallCompleted, etc.)
- The output includes the relation types "observed_in", "derived_from", "references_file", "modified_file", and "records_model_call" (or a subset, depending on the event type)
- `cargo build && cargo test` passes
- The expanded filter does NOT include every possible relation type — it should remain focused on provenance and impact, not become a duplicate of the timeline view

Verification:
- cargo test --lib state -- --test-threads=1
- cargo build
- Manual smoke: `cargo run -- state graph evidence <a-recent-event-id> --depth 2` (use an event ID from `cargo run -- state tail --limit 5`)

Expected Evidence:
- `state graph evidence` produces non-empty output
- Dashboard evidence claims gain new relation types visible in the "by relation" summary line
- Future sessions can use `state graph evidence` to trace failure provenance

Implementation:
1. In `src/commands_state.rs`, find the `graph_evidence_relation` function (around line 11578).
2. Add the runtime relation types that the SQLite projection stores for regular events:
   - "observed_in" — event belongs to a run
   - "derived_from" — event has parent events
   - "references_file" — event references a file (FileRead)
   - "modified_file" — event modified a file (FileEdited, CommitCreated, RevertPerformed)
   - "modified" — same as above (kept for backward compat in the projection)
   - "records_model_call" — event records a model call
   - "uses_model" — model call uses a specific model
   - "traced_by" — event belongs to a trace
3. Add these to the existing `matches!` macro arm list.
4. Do NOT add generic relation types that would make the evidence view identical to the timeline view. Keep the filter focused on provenance (where did this come from?), impact (what did this touch?), and association (what run/trace/model is this connected to?).
5. If there are existing tests for `build_graph_evidence_report`, update them to reflect the expanded filter.

Do not modify `state.rs` or the SQLite projection. The relations already exist in the database — the fix is purely in the filter function.
