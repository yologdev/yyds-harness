Title: Extract one self-contained module from commands_state.rs
Files: src/commands_state.rs, src/commands_state_<target>.rs (new), src/lib.rs
Issue: none
Origin: planner

Objective:
Extract at least one self-contained handler group from `commands_state.rs` (23,548 lines) into its own module, following the established extraction pattern used by `commands_state_memory.rs`, `commands_state_crashes.rs`, and `commands_state_graph.rs`.

Why this matters:
`commands_state.rs` remains 16% of the codebase despite Day 103 extracting 450 lines (→ `commands_state_memory.rs`). The extraction pattern is proven: three extractions already exist and work. Each extraction improves maintainability, reduces the blast radius of edits, and makes the module boundaries discoverable. The assessment's MEDIUM priority item flags this as the most actionable structural improvement.

Success Criteria:
- One group of handler functions (at least 200 lines, at most 800 lines) is moved from `commands_state.rs` into a new `src/commands_state_<target>.rs` file
- The new module is declared in `src/lib.rs` (alongside existing `commands_state_*` modules)
- All existing tests pass unchanged — the extraction is purely organizational
- `cargo build` and `cargo test` pass

Verification:
- cargo check
- cargo test commands_state
- cargo test --lib  (for module declaration/import validation)

Expected Evidence:
- A new `src/commands_state_<target>.rs` file exists with extracted handler code
- `commands_state.rs` is measurably smaller
- Dashboard files-changed metrics show reduction in commands_state.rs size

Implementation Notes:
- Follow the exact pattern of `commands_state_memory.rs`: move handler function(s), update the dispatch in `handle_state_subcommand()` to call the handler via module, add `use` imports, declare `mod` in lib.rs
- Pick a handler group that forms a natural unit. Candidates from the dispatch table:
  - `handle_cache` + `handle_policies` (cache & policy diagnostics)
  - `handle_evals` + `handle_patches` (eval & patch state queries)
  - `handle_failures` + `handle_fixes` + `handle_rollbacks` (error/fix tracking)
  - `handle_export` + `handle_import` (state data I/O)
  - `handle_tail` + `handle_trace` (log inspection)
  - `handle_lifecycle` + `handle_project` (lifecycle/project queries)
- Choose the group that has the clearest internal boundaries and fewest cross-references into commands_state.rs private functions/types
- If the chosen group has dependencies on `commands_state.rs` private functions, consider extracting those helpers too or using `pub(crate)` visibility
- Do NOT attempt to extract more than one group — keep it small and verifiable
- This is a pure move/extract — no behavior changes, no logic modifications
