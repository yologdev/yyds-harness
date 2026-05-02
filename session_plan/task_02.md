Title: Extract search commands from commands_search.rs into commands_ast_grep.rs
Files: src/commands_search.rs, src/commands_ast_grep.rs, src/dispatch.rs
Issue: none

## What

Split `commands_search.rs` (2,202 lines) by extracting the ast-grep related functionality into its own `commands_ast_grep.rs` module. The assessment identifies this file as one of the largest, mixing five different search commands (/find, /index, /outline, /grep, /ast) where each is self-contained.

## Why

This continues the consolidation arc that has been productive across Days 53-62. `commands_search.rs` at 2,202 lines has five distinct search command handlers. The `/ast` (ast-grep) functionality is the most self-contained — it has its own argument parser, its own runner, its own output formatting, and its only dependency on the rest of `commands_search.rs` is shared use of a few utility types. Extracting it reduces `commands_search.rs` to ~1,900 lines and gives ast-grep its own home.

## How

1. Create `src/commands_ast_grep.rs` containing:
   - `AST_GREP_FLAGS` constant
   - `is_ast_grep_available()` function
   - `run_ast_grep_search()` function
   - `parse_ast_grep_args()` function
   - `handle_ast_grep()` function
   - Any ast-grep specific imports

2. In `commands_search.rs`:
   - Remove the extracted functions
   - Add `pub use commands_ast_grep::*;` or explicit re-exports if needed for backward compatibility
   - Keep /find, /index, /outline, /grep in place

3. In `dispatch.rs`:
   - Update imports if the dispatch calls `handle_ast_grep` directly (check first — it may route through `commands_search.rs` already)

4. Add `mod commands_ast_grep;` to `main.rs`.

## Sizing

This is a mechanical extraction — move functions, update imports, verify build. No logic changes. Should be 15 minutes max.

## Testing

`cargo build && cargo test` — all existing tests must pass unchanged. If there are ast-grep specific tests in `commands_search.rs`, move those too.
