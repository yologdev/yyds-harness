Title: Extract /update command from commands_dev.rs into commands_update.rs
Files: src/commands_dev.rs, src/commands_update.rs (new), src/commands.rs, src/main.rs
Issue: none

## What to do

Extract all `/update` self-update functionality from `commands_dev.rs` into a new `commands_update.rs` file. This removes ~347 lines from the 1,693-line `commands_dev.rs`, which currently bundles 5 orthogonal concerns (/update, /doctor+/health+/fix, /watch, /tree).

### Functions to move (lines 1–362 of commands_dev.rs)

Move these to `src/commands_update.rs`:
- `handle_update()` (pub)
- `platform_asset_name()` (private)
- `is_cargo_dev_build()` (private)
- `fetch_latest_release()` (private)
- `find_asset_url()` (private)
- `download_file()` (private)
- `extract_archive()` (private)
- All related structs, constants, and `use` imports needed by these functions

### Wiring changes

1. In `src/main.rs`: Add `mod commands_update;` in the alphabetically correct position (after `commands_todo`)
2. In `src/commands.rs`: Change the `pub use crate::commands_dev::{...}` line to remove `handle_update` from commands_dev re-exports and add `pub use crate::commands_update::handle_update;`
3. Move any tests related to update functionality from commands_dev.rs to commands_update.rs

### Verification

- `cargo build` must pass
- `cargo test` must pass (all 2,413+ tests)
- `cargo clippy --all-targets -- -D warnings` must pass
- `cargo fmt -- --check` must pass
- Verify `commands_dev.rs` shrinks by ~347 lines
- Verify the new `commands_update.rs` compiles independently with correct imports
