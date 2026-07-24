Title: Improve state graph hotspots --kind error message when filter matches zero nodes
Files: src/commands_state_graph.rs
Issue: none
Origin: planner

Evidence:
- Assessment friction: `yyds state graph hotspots --kind all --limit 10` returns "no graph relations found" — not helpful when the filter simply doesn't match any node kinds
- Assessment friction: `yyds state graph hotspots --kind failure --limit 10` returns the same unhelpful message; the user can't tell whether the data is empty or their filter is wrong
- Source: `build_graph_hotspots_report()` line 1199-1201 returns `Err("no graph relations found")` for ALL empty-result cases, ignoring whether a kind filter was active
- Source: `build_graph_hotspots_payload()` line 1230-1231 has the same issue
- Valid kinds include: tool, event, patch, eval, run, trace, hypothesis, decision, evidence, policy, issue, artifact, file (from infer_graph_node_kind + dst_kind column)

Edit Surface:
- src/commands_state_graph.rs

Verifier:
- cargo build
- target/debug/yyds state graph hotspots --kind all --limit 5
- target/debug/yyds state graph hotspots --kind tool --limit 5

Fallback:
- If the error message already includes kind info (check current source), mark this task obsolete.
- If the SQLite projection has zero rows, the "no graph relations found" message is correct for all cases — add a precondition note.

Objective:
When `state graph hotspots --kind X` returns zero matching nodes, tell the user which kind they filtered by and list the kinds that DO exist in the data, so they can try a valid one.

Why this matters:
The graph hotspot command is a key diagnostic tool for understanding state graph node activity. When a filter returns nothing, the user currently gets "no graph relations found" — the same message as when the projection is empty. This makes it impossible to distinguish "your filter is wrong" from "there's no data." The fix is small: change the error message in the empty-result path to include the filter value and, when practical, list the actual kinds found in the projection.

Success Criteria:
- `yyds state graph hotspots --kind all` says something like "no hotspots matched kind=all; kinds in data: tool" instead of "no graph relations found"
- `yyds state graph hotspots --kind tool` still works correctly (returns tool hotspots)
- `yyds state graph hotspots` (no filter) with empty projection still says "no graph relations found"
- `cargo build` succeeds

Verification:
- cargo build
- target/debug/yyds state graph hotspots --kind all --limit 5
- target/debug/yyds state graph hotspots --kind nonexistent --limit 5
- target/debug/yyds state graph hotspots --kind tool --limit 5
- target/debug/yyds state graph hotspots --limit 5  (no filter, verify no regression)

Expected Evidence:
- The `--kind all` or `--kind nonexistent` output shows a helpful error with the filter value and the kinds actually present
- No regression in unfiltered or valid-filtered output

Implementation Notes:
- The change is in `build_graph_hotspots_report()` around lines 1199-1201 and `build_graph_hotspots_payload()` around lines 1230-1231
- When `hotspots.is_empty()` and `kind_filter.is_some()`, change the error to include the filter value
- Optionally: run a second query without the kind filter (or with just DISTINCT dst_kind) to discover what kinds exist in the data, and list them in the error message
- The simplest approach: add the kind to the error ("no hotspots matched kind=X") — this alone distinguishes filter-mismatch from empty-data
- A better approach: also query distinct kinds from state_relations and list them: "no hotspots matched kind=X; kinds in data: tool, file, event"
- Keep the change minimal (under 30 lines)
