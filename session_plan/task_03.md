Title: Extract help_text() from cli.rs into help.rs as the canonical source
Files: src/cli.rs, src/help.rs
Issue: none

## What to do

`cli.rs::help_text()` is a 504-line function — the largest string literal in the codebase — that generates the `--help` output. Meanwhile, `help.rs` (1,581 lines) already handles `/help` command display with `command_help()`, `help_text()`, and `handle_help()`. These two independent help systems describe the same commands but live in different files and can drift.

This task consolidates the CLI `--help` text generation into `help.rs`, making it the single source of truth for all help content.

### Specifically:

1. **Move `cli.rs::help_text()` into `help.rs`** as `pub fn cli_help_text() -> String` (or similar name to distinguish it from the existing `/help` function in help.rs).

2. **In `cli.rs`**, replace the inline `help_text()` function with a call to `help::cli_help_text()`. The `print_help()` function in cli.rs should call the new location.

3. **Verify** that `--help` output is identical before and after the move. Add a test that calls the new function and asserts it contains key expected strings (e.g., "--model", "--provider", "--prompt", "--skills").

4. **Do NOT** attempt to merge the `/help` and `--help` systems yet — that's a separate, larger task. This is purely moving the function to the right module.

### Why this matters:
- Removes 504 lines from `cli.rs` (currently 3,237 lines, the largest file)
- Groups all help-related code in one module
- Makes it easier to keep `--help` and `/help` consistent in the future

### Verification:
- `cargo build` must pass
- `cargo test` must pass
- `cargo clippy --all-targets -- -D warnings` must pass
- `yoyo --help` output should be identical (if testable, add a snapshot test)
