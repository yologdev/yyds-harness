Title: Extract symbol extraction engine from commands_map.rs into src/symbols.rs
Files: src/symbols.rs (new), src/commands_map.rs
Issue: none

## What to do

`commands_map.rs` is 4,627 lines — the largest source file. It contains two distinct responsibilities:
1. **Symbol extraction engine**: types (`SymbolKind`, `Symbol`, `FileSymbols`), language detection (`detect_language`), per-language regex extraction (`extract_symbols`), ast-grep integration (`ast_grep_rule_for_language`, `parse_ast_grep_symbols`, `extract_symbols_ast_grep`), and `MapBackend` enum.
2. **Map command handler**: `build_repo_map`, formatting, display, prompt generation, `/map` command handler.

Extract responsibility #1 into a new `src/symbols.rs` module.

### What moves to `src/symbols.rs`:
- `SymbolKind` enum (line ~228)
- `Symbol` struct (line ~245)
- `FileSymbols` struct (line ~254)
- `detect_language` function (line ~712)
- `extract_symbols` function and all per-language extraction helpers (lines ~739–1437)
- `ast_grep_rule_for_language` helper
- `parse_ast_grep_symbols` function (line ~1438)
- `extract_symbols_ast_grep` function (line ~1650)
- `MapBackend` enum (line ~1677)
- Any helper functions and constants used only by the above (like `is_binary_extension` if it's only used there)
- All `#[cfg(test)]` tests for the moved functions

### What stays in `commands_map.rs`:
- `build_repo_map`, `build_repo_map_with_backend`
- `format_repo_map`, `format_repo_map_colored`
- `recently_modified_files`
- `generate_repo_map_for_prompt`, `generate_repo_map_for_prompt_with_limit`
- `handle_map`
- Tests for the above

### Backward compatibility:
In `commands_map.rs`, add `pub use crate::symbols::*;` (or selectively re-export the public types and functions). This ensures all external consumers (`commands_search.rs`, `commands_file.rs`, `cli.rs`, `commands.rs`) continue to work without changes.

Add `mod symbols;` to `main.rs`.

### Verification:
- `cargo build` must pass
- `cargo test` must pass (all existing tests must still work)
- `cargo clippy --all-targets -- -D warnings` must pass
- The external imports (`use crate::commands_map::Symbol`, etc.) must still resolve

### Do NOT update CLAUDE.md
The CLAUDE.md project files list is auto-maintained; just ensure the code compiles and tests pass.
