Title: Extract bash safety analysis from tools.rs into src/safety.rs
Files: src/tools.rs, src/safety.rs
Issue: none

## What

Extract the bash command safety analysis subsystem from `tools.rs` (2,813 lines) into a new `src/safety.rs` module. This continues the Day 53 structural cleanup arc (which extracted `format/output.rs` and `format/diff.rs`).

## What to extract

The following functions form a self-contained unit with no dependencies on the rest of tools.rs:

1. `pub fn analyze_bash_command(command: &str) -> Option<String>` (line ~1296)
2. `fn is_at_word_boundary(s: &str, pos: usize) -> bool` (line ~1349)
3. `fn check_rm_destruction(cmd: &str) -> Option<String>` (line ~1358)
4. `fn check_git_force(cmd: &str) -> Option<String>` (line ~1402)
5. `fn check_permission_changes(cmd: &str) -> Option<String>` (line ~1427)
6. `fn check_file_overwrites(cmd: &str) -> Option<String>` (line ~1451)
7. `fn check_system_commands(cmd_lower: &str) -> Option<String>` (line ~1483)
8. `fn check_database_destruction(cmd_lower: &str) -> Option<String>` (line ~1513)
9. `fn check_pipe_from_internet(cmd_lower: &str) -> Option<String>` (line ~1540)
10. `fn check_process_killing(cmd: &str) -> Option<String>` (line ~1570)
11. `fn check_disk_operations(cmd_lower: &str) -> Option<String>` (line ~1600)

Plus ALL their associated tests from the `#[cfg(test)] mod tests` block (grep for tests that call `analyze_bash_command`, `check_rm_destruction`, etc.).

## How

1. Create `src/safety.rs` with the extracted functions and their tests
2. In `tools.rs`, remove the extracted functions and add `use crate::safety::analyze_bash_command;`
3. Add `mod safety;` in `main.rs`
4. Keep `analyze_bash_command` as `pub` — it's used in `tools.rs` (in `StreamingBashTool`)
5. The helper functions (`check_rm_destruction`, etc.) can stay `pub(crate)` or private in the new module
6. Run `cargo build && cargo test` to verify nothing breaks
7. Run `cargo clippy --all-targets -- -D warnings` and `cargo fmt`

## Verification

- `cargo test` — all existing safety analysis tests must pass in their new location
- `cargo clippy --all-targets -- -D warnings` — clean
- `tools.rs` should shrink by ~350 lines of code + ~1000 lines of tests

## Docs

Update CLAUDE.md:
- Add `src/safety.rs` to the Architecture section with description: "bash command safety analysis, destructive pattern detection"
- Update `tools.rs` description to remove "bash command safety analysis" mention
