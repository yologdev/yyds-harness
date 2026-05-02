Title: Extract rename subsystem from commands_refactor.rs into commands_rename.rs
Files: src/commands_refactor.rs, src/commands_rename.rs
Issue: none

## What to do

`commands_refactor.rs` is 2,719 lines with three distinct subsystems: extract, rename, and move. Extract the rename subsystem into its own `src/commands_rename.rs` module.

### Code to extract

Move the following from `commands_refactor.rs` to `src/commands_rename.rs`:

**Helper functions** (these are used only by rename):
- `fn is_word_boundary_char(c: char) -> bool` (line ~393)
- `fn is_word_start(text: &str, pos: usize) -> bool` (line ~400)
- `fn is_word_end(text: &str, pos: usize) -> bool` (line ~413)

**Structs:**
- `pub struct RenameMatch` (line ~425)
- `pub struct RenameResult` (line ~434)

**Functions:**
- `pub fn rename_in_project(...)` (line ~444)
- `pub fn find_rename_matches(old_name: &str) -> Vec<RenameMatch>` (line ~493)
- `pub fn find_word_boundary_matches(text: &str, pattern: &str) -> Vec<usize>` (line ~528)
- `fn list_git_files() -> Vec<String>` (line ~562)
- `pub fn format_rename_preview(...)` (line ~581)
- `pub fn apply_rename(...)` (line ~625)
- `pub fn replace_word_boundary(text: &str, old: &str, new: &str) -> String` (line ~669)
- `pub fn parse_rename_args(input: &str) -> Option<(String, String)>` (line ~701)
- `pub fn handle_rename(input: &str)` (line ~713)

**Tests** — move all rename-related tests:
- Search for test functions containing `rename`, `word_boundary`, `is_word_start`, `is_word_end`, `RenameMatch` in their body/name
- These include `test_parse_rename_args_*`, `test_rename_*`, `test_find_rename_matches_*`, `test_word_boundary_*`, `test_replace_word_boundary_*`, `test_apply_rename_*`, `test_format_rename_preview_*`, and any multi-byte safety tests for rename functions

### New file structure

`src/commands_rename.rs`:
- Module doc: `//! Rename symbol across project files — word-boundary-aware find-and-replace`
- Bring over needed imports (look at what the extracted functions use: `std::fs`, `std::path::Path`, `crate::format::*`, `crate::git::run_git` if used)
- Keep all `pub` functions `pub`
- Move related tests into `#[cfg(test)] mod tests { ... }`

### In commands_refactor.rs

- Remove all rename code and tests
- Add `use crate::commands_rename::handle_rename;` (the only rename function called from the routing `handle_refactor`)
- The file should now contain only: extract subsystem + move subsystem + `handle_refactor` routing + their tests

### In main.rs

- Add `mod commands_rename;`

### In tools.rs (if applicable)

- Check if `RenameSymbolTool` references any rename functions: `grep -n "rename_in_project\|find_rename_matches\|apply_rename" src/tools.rs`
- Update imports if needed

### Update CLAUDE.md

Add to the module list:
```
- `commands_rename.rs` — rename symbol across project files, word-boundary matching, preview and apply
```

### Verify

- `cargo build && cargo test`
- `cargo clippy --all-targets -- -D warnings`
