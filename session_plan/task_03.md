Title: Extract move subsystem from commands_refactor.rs into commands_move.rs
Files: src/commands_refactor.rs, src/commands_move.rs
Issue: none

## What to do

After Task 2 extracts the rename subsystem, `commands_refactor.rs` will have two remaining subsystems: extract and move. Extract the move subsystem into `src/commands_move.rs`, leaving `commands_refactor.rs` with only the extract subsystem and the `handle_refactor` routing function.

**Important**: This task runs AFTER Task 2 (rename extraction). The line numbers below are from the original file — verify them in the post-Task-2 state.

### Code to extract

Move the following from `commands_refactor.rs` to `src/commands_move.rs`:

**Structs:**
- `pub struct MoveArgs` (originally line ~771)
  
**Functions:**
- `pub fn parse_move_args(input: &str) -> Option<MoveArgs>` (originally line ~779)
- `pub fn find_impl_blocks(source: &str, type_name: &str) -> Vec<(usize, usize, String)>` (originally line ~821)
- `pub fn find_method_in_impl(...)` (originally line ~922)
- `pub fn move_method(...)` (originally line ~1009)
- `fn reindent_method(method_text: &str, target_indent: &str) -> String` (originally line ~1186)
- `pub fn handle_move(input: &str)` (originally line ~1219)
- `fn find_file_with_impl(type_name: &str) -> Option<String>` (originally line ~1363)

**Tests** — move all move-related tests:
- Search for test functions containing `move_method`, `move_args`, `find_impl_blocks`, `find_method_in_impl`, `reindent_method`, `MoveArgs`, `handle_move`, `find_file_with_impl`
- These include `test_parse_move_args_*`, `test_find_impl_blocks_*`, `test_find_method_in_impl_*`, `test_move_method_*`, `test_reindent_method*`, and any multi-byte safety tests for move functions

### New file structure

`src/commands_move.rs`:
- Module doc: `//! Move methods between impl blocks — cross-file method relocation`
- Bring over needed imports (look at what the extracted functions use: `std::fs`, `std::path::Path`, `crate::format::*`, `crate::git::run_git` if used)
- Keep all `pub` functions `pub`
- Move related tests into `#[cfg(test)] mod tests { ... }`

### In commands_refactor.rs

- Remove all move code and tests
- Add `use crate::commands_move::handle_move;`
- The file should now contain only: extract subsystem (`parse_extract_args`, `find_symbol_block`, `extract_symbol`, `handle_extract`) + `handle_refactor` routing + extract-related tests
- This should bring `commands_refactor.rs` down from ~2,719 lines to ~800-1000 lines

### In main.rs

- Add `mod commands_move;`

### Update CLAUDE.md

Add to the module list:
```
- `commands_move.rs` — move methods between impl blocks, cross-file method relocation
```

### Verify

- `cargo build && cargo test`
- `cargo clippy --all-targets -- -D warnings`
- Verify `commands_refactor.rs` is now under 1,000 lines
