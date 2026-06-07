# Task 03: Extract state memory handlers from commands_state.rs into commands_state_memory.rs

Title: Extract state memory handlers into their own module
Files: src/commands_state.rs, src/commands_state_memory.rs, src/lib.rs
Issue: none

## Problem

`commands_state.rs` is 23,871 lines — 16.7% of the entire codebase. The state command dispatcher (`handle_state_subcommand`) routes to ~25 subcommand handlers, each substantial. The `commands_state_graph.rs` module was already extracted (1,168 lines). This task extracts the memory subcommand handlers, following the same pattern.

## What To Do

1. **Create `src/commands_state_memory.rs`** with the following functions moved from `commands_state.rs`:
   - `handle_memory(args)` — the subcommand dispatcher (line ~574)
   - `print_memory_usage()` — usage text helper (line ~584)
   - `handle_memory_synthesize(args)` — synthesize handler (line ~593)
   - `handle_memory_list(args)` — list handler (line ~644)
   - `handle_memory_decision(args, promote)` — promote/reject handler (line ~678)

   These functions are approximately lines 574-707 in `commands_state.rs`. They are self-contained and only depend on shared utility functions like `default_events_path()`, `read_events()`, `flag_value()`, `preview_line()`, `build_state_memory_synthesis()`, `build_state_memory_records()`, `record_state_memory_candidates()`, `record_state_memory_decision()`, and `write_text_artifact()`. These utilities stay in `commands_state.rs` (they're shared across subcommand handlers).

2. **Update `src/commands_state.rs`**:
   - Remove the 5 functions above (~135 lines)
   - In `handle_state_subcommand`, change the `"memory"` match arm to call `crate::commands_state_memory::handle_memory(args)` (similar to how `"graph"` calls `crate::commands_state_graph::handle_graph_subcommand`)

3. **Update `src/lib.rs`**:
   - Add `mod commands_state_memory;` after the existing `mod commands_state_graph;` (line ~74)

4. **Add a `pub` visibility** to `handle_memory` in the new module so it can be called from `commands_state.rs`.

## Verification

- `cargo build` must pass with no warnings
- `cargo test` must pass (89 tests)
- `cargo clippy --all-targets -- -D warnings` must pass
- Run `yyds state memory` — should print usage text
- Run `yyds state memory list` — should work (may show "no state log found" if no state exists, that's fine)
- Run `yyds state memory synthesize` — should work or show appropriate message
- Verify the old dispatch still works: `yyds state memory` shows usage, `yyds state memory list --status proposed` works

## Sizing Note

This is a pure code move — no logic changes. ~135 lines moved, 3 files touched. This is the smallest and most self-contained extraction from `commands_state.rs`. Future tasks can extract other handler groups (journal, export/import, retention, etc.) following the same pattern.
