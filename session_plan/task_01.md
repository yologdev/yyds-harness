Title: Extract help_data.rs from help.rs — move static command data to dedicated module
Files: src/help.rs, src/help_data.rs (new), src/main.rs
Issue: none

## What

Extract the two massive static-data match functions from `help.rs` into a new `src/help_data.rs` module:

1. **`command_help(cmd: &str) -> Option<&'static str>`** — lines ~37-1175, ~1,138 lines. A pure match statement mapping command names to detailed help strings.
2. **`command_short_description(cmd: &str) -> Option<&'static str>`** — lines ~2004-3397, ~1,393 lines. A pure match statement mapping command names to one-line descriptions.

## Why

`help.rs` is 3,397 lines — the second-largest file in the codebase. Two-thirds of it is static string data (match arms returning help text). Extracting these into `help_data.rs` reduces `help.rs` to ~866 lines of actual logic: completions, help rendering, CLI help text builder, and command handlers.

This follows the same pattern as Day 81's `symbols.rs` extraction from `commands_map.rs` — separating "what data exists" from "how to use it."

## How

1. Create `src/help_data.rs` with the two functions moved verbatim from `help.rs`
2. In `help.rs`, replace the moved functions with `pub use help_data::{command_help, command_short_description};` (or call through)
3. Add `mod help_data;` to `src/main.rs`
4. The functions in `help.rs` that call `command_help` and `command_short_description` should continue to work unchanged via the re-export
5. Run `cargo build && cargo test` to verify nothing breaks
6. Keep all existing tests in their current locations — tests in `help.rs` that test `command_help` or `command_short_description` can stay where they are since the re-export makes them accessible

## Verification

- `cargo build` clean
- `cargo test` passes (all existing help tests should pass unchanged)
- `cargo clippy --all-targets -- -D warnings` clean
- `help.rs` drops from ~3,397 to ~866 lines
- `help_data.rs` is ~2,531 lines (pure data)
- No functionality change whatsoever

## CLAUDE.md update

Add `src/help_data.rs` to the architecture section under the `help.rs` entry:
- `help_data.rs` — static command help text and short descriptions (pure data, extracted from `help.rs`)
