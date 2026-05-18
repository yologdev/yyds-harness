Title: Permission persistence for file operations — offer to save patterns on "always"
Files: src/tool_wrappers.rs, src/tools.rs
Issue: none

## Context

Bash commands already have `offer_persist_pattern()` in `tools.rs` — when a user says "always" to
approve all bash commands, it offers to save the simplified pattern (e.g., `cargo test*`) to
`.yoyo.toml`'s `[permissions] allow` list. File operations (write_file, edit_file) do NOT have
this. When users say "always" on the ConfirmTool prompt, it sets the in-memory `always_approved`
AtomicBool but never offers to persist.

This is the #1 UX friction gap vs Claude Code — permission persistence across sessions. Claude Code
remembers what you've approved. yoyo re-asks every session for file operations.

## What to do

1. In `tool_wrappers.rs`, modify `confirm_file_operation()`:
   - After the user says "a" or "always" and the `always_approved` flag is set, call a new
     helper to offer persisting a *directory pattern* to `.yoyo.toml`.
   - The pattern should be based on the file path: extract the directory part and create a
     glob like `src/*` or `tests/*`. If the file is in the project root, use `*.rs` or similar.
   - Use `crate::config::append_allow_pattern` to persist (same mechanism as bash).

2. Add a helper function `offer_persist_file_pattern(path: &str)` in `tool_wrappers.rs` that:
   - Extracts the directory from the path
   - Creates a pattern like `src/*` (directory-based) or `*.ext` (root files)
   - Uses a session-level dedup mechanism (like `already_offered_persistence` in tools.rs)
     to avoid repeatedly asking for the same directory
   - Prompts the user: `Save 'src/*' to .yoyo.toml allow list? (y/n)`
   - On "y", calls `crate::config::append_allow_pattern(&pattern)`

3. Add tests:
   - Test that `offer_persist_file_pattern` generates correct patterns from paths
   - Test that the dedup mechanism works (second call for same dir doesn't re-prompt)
   - Test that `confirm_file_operation` returns true when `always_approved` is set (already exists)

## Important notes

- Do NOT move or refactor `offer_persist_pattern` from tools.rs — just add the parallel for file ops
- The bash version in tools.rs uses `simplify_command_pattern` — the file version needs a different
  pattern generator based on directory paths, not command tokens
- Keep the dedup static (LazyLock<Mutex<HashSet<String>>>) like the bash version
- Pattern should be the directory path + `/*`, e.g., `src/format/*` for `src/format/mod.rs`
