Title: Extract subcommand dispatch from dispatch.rs into dispatch_sub.rs
Files: src/dispatch.rs, src/dispatch_sub.rs, src/main.rs
Issue: none

`dispatch.rs` is 1,655 lines with 189 match arms — the biggest remaining structural debt. The file contains two distinct concerns:

1. **`try_dispatch_subcommand()`** (lines ~101-430): CLI subcommand routing for `yoyo <subcmd>` (doctor, health, help, version, setup, init, lint, test, tree, map, outline, run, diff, commit, review, blame, grep, find, index, update, docs, skill, watch, status, undo, changelog, evolution, config, permissions, todo, memories, extended). ~330 lines.

2. **`dispatch_command()`** (lines ~434-1655): REPL `/command` routing for interactive session commands. ~1,220 lines.

These are completely independent — one runs before the REPL starts (synchronous, returns `Option<Config>`), the other runs inside the REPL loop (async, returns `CommandResult`).

## Task

Extract `try_dispatch_subcommand()` and its helper functions (`quote_args_as_command`, `flag_value`, `FlagValueCheck`, `require_flag_value`) into a new file `src/dispatch_sub.rs`.

### Steps

1. Create `src/dispatch_sub.rs` with the extracted functions
2. Add `pub(crate) mod dispatch_sub;` to `main.rs`
3. Update `dispatch.rs` to remove the extracted code, keeping only `dispatch_command()` and `CommandResult`/`DispatchContext`
4. Update any imports in `dispatch.rs` that referenced the moved functions
5. Update `main.rs` to call `dispatch_sub::try_dispatch_subcommand()` instead of `dispatch::try_dispatch_subcommand()`
6. Verify with `cargo build && cargo test && cargo clippy --all-targets -- -D warnings`

### Size guard

This is a clean mechanical extraction — the two functions don't share any state, only type imports. The extraction should NOT change any logic, only move code to a new file. If any test references `dispatch::try_dispatch_subcommand` directly, update the import path.

### What NOT to do

- Don't refactor the match arms themselves — this is a pure extraction
- Don't change any command behavior
- Don't rename any functions (except adding the module path prefix)
- Don't split `dispatch_command()` further — that's a separate task
