Title: Fix `state graph hotspots --kind failure` filter not filtering
Files: src/commands_state_graph.rs
Issue: none
Origin: planner

Evidence:
- Assessment self-test: `yyds state graph hotspots --limit 10` and `yyds state graph hotspots --kind failure --limit 10` returned identical results (bash/read_file/search tools), proving the `--kind` flag is not filtering.
- `handle_graph_hotspots(limit, args.iter().any(|arg| arg == "--json"))` at line 451 only parses `--json` — no `--kind` parsing.
- `query_graph_hotspots()` at line 1245 queries all `state_relations` rows without a `WHERE dst_kind = ?` clause.
- The `GraphHotspot` struct at line 1176 already has a `kind: String` field — the data model supports filtering, it's just not implemented.

Edit Surface:
- src/commands_state_graph.rs

Verifier:
- cargo build && cargo test state_graph -- --test-threads=1
- target/debug/yyds state graph hotspots --kind failure --limit 5

Fallback:
- If the `--kind` flag is intentionally a no-op (deprecated, placeholder), add a deprecation warning to stderr and update the help text.
- If `dst_kind` values don't include "failure", check what values exist in the projection and pick the correct column/filter.

Objective:
Make `yyds state graph hotspots --kind failure` show only failure-related hotspots (tool errors, model errors, crashes) instead of all hotspots.

Why this matters:
The `--kind` flag is advertised in help text but silently ignored, making failure diagnosis harder. When a session has tool failures, `state graph hotspots --kind failure` should surface the failure hotspots without mixing in normal tool usage (bash, read_file, search). This is a concrete, verifiable bug with no blocked dependencies.

Success Criteria:
- `yyds state graph hotspots --kind failure --limit 5` returns hotspots whose `kind` is "failure" (or matching event kinds like tool_error, model_error)
- `yyds state graph hotspots --limit 5` (no --kind) continues to work as before, showing all hotspots
- `cargo test state_graph` passes

Verification:
- cargo build
- cargo test state_graph -- --test-threads=1
- target/debug/yyds state graph hotspots --kind failure --limit 5

Expected Evidence:
- After fix: `state graph hotspots --kind failure` shows different (narrower) results than without the flag
- The hotspot `kind` field in output shows "failure" or related error kinds
- No regression in unfiltered hotspots output

Implementation Notes:
- Parse `--kind` from args in the dispatch block around line 447-452 using the existing `flag_value` helper (used elsewhere in the same file for `--limit`, `--depth`, etc.)
- Pass `kind_filter: Option<&str>` through `handle_graph_hotspots` → `build_graph_hotspots_report`/`build_graph_hotspots_payload` → `query_graph_hotspots`
- In `query_graph_hotspots`, when `kind_filter` is `Some(kind)`, add `WHERE dst_kind = ?` to the SQL query and bind the parameter
- The SQL query currently selects `dst_kind` from `state_relations` — filter on that column
- Keep the change minimal: ~10-20 lines across 3-4 functions
- Do NOT refactor the function signatures beyond adding the `kind_filter` parameter
