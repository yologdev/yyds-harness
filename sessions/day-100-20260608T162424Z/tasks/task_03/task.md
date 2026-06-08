Title: Extract graph hotspots functions from commands_state.rs to commands_state_graph.rs
Files: src/commands_state.rs, src/commands_state_graph.rs
Issue: none
Origin: planner

Objective:
Move the `build_graph_hotspots_report` and `build_graph_hotspots_payload` functions (plus any private helpers they depend on) from `src/commands_state.rs` (23,736-line monolith) to `src/commands_state_graph.rs` (1168 lines, already contains `handle_graph_subcommand`). This is the first step in splitting the monolith — one self-contained function group, independently verifiable, minimal risk.

Why this matters:
`commands_state.rs` is 23,736 lines (16.5% of codebase) — 3.6x larger than the next-largest file. This was flagged in Day 99 assessment and hasn't been addressed. Monolithic files make editing risky (merge conflicts, accidental breakage) and slow down evolution sessions (more context consumed to find things). The graph subsystem is the natural split target because `commands_state_graph.rs` already exists as the dispatcher module. Moving one function group at a time is safe and verifiable.

Success Criteria:
- `build_graph_hotspots_report` and `build_graph_hotspots_payload` live in `commands_state_graph.rs`
- `commands_state.rs` no longer contains these functions
- `cargo build` passes
- `cargo test --lib` passes
- `yyds state graph hotspots --limit 10` produces identical output to before the move
- `commands_state.rs` line count decreases (even if modestly)

Verification:
- cargo build (must pass)
- cargo test --lib (must pass, especially any state graph tests)
- yyds state graph hotspots --limit 10 (must produce identical output)
- grep -n "build_graph_hotspots" src/commands_state.rs (should return nothing)
- grep -n "build_graph_hotspots" src/commands_state_graph.rs (should find the functions)

Expected Evidence:
- commands_state.rs line count decreases
- commands_state_graph.rs line count increases
- state graph hotspots subcommand continues to work identically
- task lineage shows file edits to both files

Detailed plan:

1. Read `src/commands_state_graph.rs` to understand:
   - Current module structure (pub mod? inline?)
   - How `handle_graph_subcommand` calls the build functions
   - What imports are already present

2. Read `src/commands_state.rs` to find:
   - `build_graph_hotspots_report` function (full body)
   - `build_graph_hotspots_payload` function (full body)
   - Any private helper functions they call that are only used by hotspots
   - Any use/import statements that the hotspots functions depend on

3. Move strategy:
   - Copy the functions to `commands_state_graph.rs` (after existing code, with appropriate visibility)
   - Add any needed imports to `commands_state_graph.rs`
   - Remove the functions from `commands_state.rs`
   - If the functions were called from `commands_state.rs` (e.g., in a dispatch table), update the call sites to reference the new location — BUT first check if they're already called via `commands_state_graph::build_graph_hotspots_report` pattern
   - If they're called within `commands_state.rs` itself, add a `use` import to pull them from `commands_state_graph`

4. Key constraint checks:
   - Check if `commands_state_graph` is a `mod` declared in lib.rs (likely `pub mod commands_state_graph;`)
   - Check if the functions are `pub` or `pub(crate)` — they need to be accessible from `commands_state.rs` if called from there
   - Check if the functions reference types defined in `commands_state.rs` — if so, those types may need to be made `pub` or re-exported

5. After moving:
   - Run cargo build — fix any compilation errors (missing imports, visibility issues)
   - Run cargo test --lib — fix any test failures
   - Run yyds state graph hotspots --limit 10 — verify identical output
   - Run cargo fmt

6. If the functions share private helpers with other graph functions still in commands_state.rs:
   - Leave the helpers in commands_state.rs and make them `pub(crate)`
   - Or duplicate small helpers if they're trivial (prefer no duplication)
   - If this creates too much entanglement, document it and move on — the hotspots functions alone are the target

DO NOT:
- Move more than the hotspots functions + their direct private helpers
- Change function signatures or behavior
- Touch any other graph functions (evals, files, commits, etc.)
- Touch more than 2 source files
- Spend more than 20 minutes — if entangled, document and stop
