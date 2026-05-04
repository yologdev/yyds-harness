Title: Extract /tree command from commands_dev.rs into commands_tree.rs
Files: src/commands_dev.rs, src/commands_tree.rs (new), src/commands.rs, src/main.rs
Issue: none

## What to do

Extract the `/tree` command and its helpers from `commands_dev.rs` into a new `commands_tree.rs` file. After Tasks 1 and 2 have removed /update (~347 lines) and /watch (~150 lines), this extraction removes another ~100 lines, completing the decomposition of commands_dev.rs into focused modules. What remains in commands_dev.rs will be the cohesive /doctor + /health + /fix trio (~700 lines), which belong together.

### Functions to move

Move these to `src/commands_tree.rs`:
- `build_project_tree(max_depth: usize) -> String` (pub)
- `format_tree_from_paths(paths: &[String], max_depth: usize) -> String` (pub)
- `handle_tree(input: &str)` (pub)
- Any related tests for tree functionality

### Wiring changes

1. In `src/main.rs`: Add `mod commands_tree;` in alphabetical position
2. In `src/commands.rs`: Move `handle_tree` from the `commands_dev` re-export to `pub use crate::commands_tree::{build_project_tree, format_tree_from_paths, handle_tree};`
3. Check if `build_project_tree` or `format_tree_from_paths` are used elsewhere (e.g., in context.rs or commands_project.rs) — if so, update those imports to use `crate::commands_tree::` directly or ensure they go through the `commands` re-export.
4. Add necessary imports to `commands_tree.rs` (format constants, filesystem operations, etc.)

### Important: Check cross-references

Before extracting, check for callers of `build_project_tree` and `format_tree_from_paths` outside of dispatch:

```bash
grep -rn "build_project_tree\|format_tree_from_paths" src/ --include="*.rs"
```

If these are called from other modules, make sure those references work after the move.

### Verification

- `cargo build` must pass
- `cargo test` must pass
- `cargo clippy --all-targets -- -D warnings` must pass  
- `cargo fmt -- --check` must pass
- After all 3 tasks, `commands_dev.rs` should be ~700-800 lines (down from 1,693), containing only the cohesive /doctor + /health + /fix concerns
- The new `commands_tree.rs` should be ~100 lines
