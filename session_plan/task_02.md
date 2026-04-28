Title: Extract /loop and /run handlers from commands_dev.rs into commands_run.rs
Files: src/commands_dev.rs, src/commands_run.rs, src/dispatch.rs
Issue: none

## Problem

`commands_dev.rs` is at 2,853 lines and contains 8+ distinct command handlers (doctor,
health, fix, test, lint, lint-fix, lint-unsafe, watch, tree, run, loop, update). The
assessment notes it's "approaching the same bloat that `main.rs` had before the Day 58
extraction." Following the consolidation pattern that's been working well (extracting
`agent_builder.rs`, `watch.rs`, `session.rs`, `safety.rs`, `dispatch.rs`), split out
the most self-contained handlers.

## What to extract

Move these into a new `src/commands_run.rs`:
- `handle_run` and `handle_run_usage` — the `/run` command handler
- `run_shell_command` — shared helper used by /run
- `LoopMode`, `parse_loop_args`, `handle_loop` — the `/loop` command (added Day 59 morning)

These are the most self-contained group — they share `run_shell_command` between them
and have minimal dependencies on other commands_dev functions.

## Implementation steps

1. Create `src/commands_run.rs` with the extracted functions
2. Add `pub(crate)` visibility where needed
3. Add `mod commands_run;` to `main.rs`
4. Update imports in `dispatch.rs` to point to new module
5. Remove the moved functions from `commands_dev.rs`
6. Run `cargo build && cargo test` to verify

Do NOT change any function signatures or behavior. This is a pure extraction —
same code, new file, new address.

Also update CLAUDE.md's Repository Structure section to list the new file.
