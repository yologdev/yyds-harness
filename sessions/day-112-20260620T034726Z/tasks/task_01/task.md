Title: Fix state event type classification in doctor — all events show as "unknown"
Files: src/commands_state.rs
Issue: none
Origin: planner

Evidence:
- `yyds state doctor` output this session: "Types: unknown=36124" for all 36K events.
- `head -1 .yoyo/state/events.jsonl` shows `"event_type": "PatchEvaluated"` — the field is named `event_type`, not `type`.
- `handle_doctor()` at line 154 reads `ev.get("type")` which never matches, so every event falls through to "unknown".
- `compatibility_event_value()` in `src/state.rs` line 3174 serializes events with `"event_type": event_type_label(&event.event_type)`, confirming the canonical field name.

Edit Surface:
- src/commands_state.rs (line 154)

Verifier:
- cargo build && yyds state doctor (should show actual type names, not all "unknown")

Fallback:
- If the fix doesn't resolve the issue, check if legacy events use a different field name ("type") and add a fallback. If both conventions exist, read "event_type" first, fall back to "type", then "unknown".

Objective:
Make `yyds state doctor` show real event type distribution instead of reporting all 36K+ events as "unknown", restoring event-type filtering and aggregation accuracy.

Why this matters:
The state event type is the primary classification axis for diagnostic queries. When all events show as "unknown", event-type filtering (`state why <id>`, `state failures`, type-specific graph queries) is non-functional. This is a field-name mismatch between the JSON serialization (`event_type`) and the read path (`type`). Fixing it restores a core observability primitive.

Success Criteria:
- `yyds state doctor` output shows real type names: RunStarted=N, ToolCallStarted=N, FileRead=N, etc. — not "unknown=36124".
- The `group_event_types` output includes multiple type groups, not a single "unknown" group.

Verification:
- cargo build
- yyds state doctor 2>&1 | grep "Types:" — should contain real type names, not "unknown=36124"
- cargo test commands_state

Expected Evidence:
- State doctor output shows typed distribution.
- Future assessments can cite specific type counts.
- State/dashboard type-level aggregation becomes functional.

Implementation Notes:
- The fix is a one-line change at `src/commands_state.rs` line 154.
- Change: `ev.get("type")` → `ev.get("event_type")`.
- Also check `payload_str()` at line 12814 which reads `.get("type")` from nested payload values — that's for extracting type info from sub-objects and may be correct as-is. Only fix the `handle_doctor()` path.
- Verify with `cargo test` since there may be tests asserting "unknown" counts that need updating.
