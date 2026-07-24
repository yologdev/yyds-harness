Title: Add failure relations to state graph projection so --kind failure returns data
Files: src/state.rs
Issue: none
Origin: planner

Evidence:
- `yyds state graph hotspots --kind failure` returns zero results: "no hotspots matched kind=failure; kinds in data: artifact, eval, event, file, model..."
- The `project_failure` function (state.rs:1547) inserts into the `failures` table but never creates graph relations
- The `project_relations` function (state.rs:1694) handles FileRead/FileEdited/ModelCall/ToolCall events but has no match arm for FailureObserved, JsonOutputFailure, or ToolSchemaFailure
- `query_graph_hotspots` filters by `h.kind`, and failure events get kind="event" from `infer_graph_node_kind` (commands_state.rs:13391) because their IDs start with "evt-". No node ever gets kind="failure".
- Graph pressure: `bash_tool_error=13` — these failures are invisible in the graph. Graph hotspots can't show which tools fail most.
- This was noted in the assessment: "[MEDIUM] `state graph hotspots --kind failure` returns zero results — the kind exists in the data schema but no failure relations are actually created"

Edit Surface:
- src/state.rs

Verifier:
- cargo test state
- cargo build && target/debug/yyds state graph hotspots --kind failure --limit 5
  (after rebuilding the projection from existing events: yyds --eval "yyds state rebuild" or similar)

Fallback:
- If the failure payloads in existing events don't carry a "source" field, use the event_type label ("FailureObserved") as the relation source identifier and still create a relation so the failure node gets kind="failure" in the graph.
- If the SQLite projection needs a manual rebuild to pick up the new relation type, note that in the task outcome — the code change is correct even if the CI-prebuilt projection doesn't immediately show the new data.

Objective:
Make `yyds state graph hotspots --kind failure` return actual data by creating graph relations for failure events. Each failure event should appear as a graph node with kind="failure", linked to the tool or source that caused the failure.

Why this matters:
The trajectory graph pressure reports `bash_tool_error=13` but `state graph hotspots --kind failure` returns zero results. This means the graph — our primary diagnostic tool — is blind to the most common failure class. Making failures visible in the graph enables:
- Hotspot queries that show which tools/files have the most failures
- State graph path traversal from failures to related events
- Future automated diagnosis of recurring failure patterns

Success Criteria:
- `state graph hotspots --kind failure` returns at least one node with kind="failure" after the projection is rebuilt with existing events
- The failure node has relations to the tool that caused it (e.g., "bash", "read_file") when the failure payload carries a "source" field
- `cargo test state` passes with no regressions
- The change is ~10-20 lines of Rust

Verification:
- cargo test state
- cargo build
- Rebuild projection: target/debug/yyds --eval "state rebuild" (or equivalent command to regenerate the SQLite projection from events)
- target/debug/yyds state graph hotspots --kind failure --limit 5 | grep failure

Expected Evidence:
- State graph contains failure→tool and failure→file relations
- `state graph hotspots --kind failure` shows nodes with kind="failure"
- Future trajectory graph pressure about bash tool errors links back to actual graph evidence

Implementation Notes:

The change is in `src/state.rs`, function `project_relations` (line 1694). Add a new block after the ToolCall block (around line 1825) that matches `FailureObserved | JsonOutputFailure | ToolSchemaFailure`:

```rust
if matches!(
    event.event_type,
    EventType::FailureObserved | EventType::JsonOutputFailure | EventType::ToolSchemaFailure
) {
    // Create a relation from the failure source (tool/operation) to the
    // failure event so the failure node appears with kind="failure" in the
    // graph, making --kind failure filtering work.
    let source = payload_str(&event.payload, "source")
        .or_else(|| payload_str(&event.payload, "operation"))
        .unwrap_or("unknown");
    insert_relation(
        conn,
        source,
        "produced_failure",
        &event.event_id,
        "failure",
        &event.event_id,
        report,
    )?;
}
```

This creates: `tool:bash --produced_failure--> evt-failure-123` where `dst_kind="failure"`.
In `query_graph_hotspots`, the destination node `evt-failure-123` gets `kind = "failure"` from `dst_kind`, making it filterable by `--kind failure`.

Key design decisions:
- The relation direction is `tool → failure` (tool produced the failure), not `failure → tool`. This makes both nodes useful: the tool node shows outgoing relations to failures, and the failure node gets kind="failure" via dst_kind.
- Use "produced_failure" as the relation name (consistent with existing naming like "records_model_call", "records_tool_call").
- The "source" field is already read by `project_failure` at line 1568 — it's the most reliable field for identifying what caused the failure.

Do NOT change `infer_graph_node_kind` in `commands_state.rs` — the approach of using `dst_kind` from the relation is cleaner and avoids adding failure-specific logic to the general-purpose ID classifier.
