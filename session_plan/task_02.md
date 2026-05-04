Title: Move /watch detection and handler from commands_dev.rs into watch.rs
Files: src/commands_dev.rs, src/watch.rs, src/commands.rs
Issue: none

## What to do

Move the `/watch` command handler and its detection helpers from `commands_dev.rs` into `watch.rs`, which already contains the watch runtime logic (set/get/clear watch commands, run_watch_command, build_watch_fix_prompt, run_watch_after_prompt). The handler and detection functions belong with the rest of the watch system.

### Functions to move (from commands_dev.rs)

Move these to `src/watch.rs`:
- `auto_detect_watch_command()` (pub) — detects appropriate watch/test command for the project
- `detect_watch_all_command()` (pub) — detects a combined lint+test command
- `detect_watch_all_phases()` (pub) — detects multi-phase watch commands
- `handle_watch(input: &str)` (pub) — the `/watch` slash command handler
- `WATCH_SUBCOMMANDS` (pub const) — the subcommand list for tab completion

### Wiring changes

1. In `src/commands.rs`: Move `handle_watch` from the `commands_dev` re-export block to a new `pub use crate::watch::{handle_watch};` line (or add to existing watch imports if any). Also update the WATCH_SUBCOMMANDS reference in `command_arg_completions` from `crate::commands_dev::WATCH_SUBCOMMANDS` to `crate::watch::WATCH_SUBCOMMANDS`.
2. Remove the moved functions from `commands_dev.rs` along with any imports that are no longer needed there.
3. Add any imports that `watch.rs` needs to support the moved functions (e.g., format constants like `DIM`, `RESET`, `GREEN`, `YELLOW`, file system operations).

### Important details

- `handle_watch` in commands_dev.rs calls `set_watch_command`, `set_watch_commands`, `get_watch_command`, `clear_watch_command` which are already in watch.rs — so the move eliminates cross-module calls.
- `auto_detect_watch_command` and related detection functions check for project files (Cargo.toml, package.json, etc.) to determine the right test/watch command.
- Move any related tests from commands_dev.rs to watch.rs.

### Verification

- `cargo build` must pass
- `cargo test` must pass
- `cargo clippy --all-targets -- -D warnings` must pass
- `cargo fmt -- --check` must pass
- Verify `commands_dev.rs` shrinks by ~150 lines
- Verify `watch.rs` gains the moved functions and they work correctly
