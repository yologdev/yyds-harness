Title: /watch multi-command support — run lint AND test in sequence
Files: src/prompt.rs, src/commands_dev.rs
Issue: none

## What to do

Currently `/watch` only supports a single command. Aider runs both linter and tests after edits. This task adds `/watch --all` which auto-detects and runs both the project's lint command AND test command in sequence, stopping at the first failure.

### Implementation

1. **In `src/prompt.rs`**: Change the `WATCH_COMMAND` from `RwLock<Option<String>>` to `RwLock<Option<Vec<String>>>` — a list of commands to run in sequence. Update:
   - `set_watch_command(cmd: &str)` → sets a single-command vec
   - Add `set_watch_commands(cmds: Vec<String>)` → sets multiple commands
   - `get_watch_command() -> Option<String>` → keep for backward compat (returns first command, or all joined with " && ")
   - Add `get_watch_commands() -> Option<Vec<String>>` → returns the full list
   - `run_watch_command(cmd: &str)` → keep as-is (runs one command)
   - Update the watch loop in `repl.rs` (line ~646) to iterate over commands, running each in sequence, stopping at the first failure

   Actually, simpler approach: keep the storage as `Option<String>` but support `&&`-chained commands. The `run_watch_command` already runs via shell so `cargo clippy -- -D warnings && cargo test` will work natively.

2. **In `src/commands_dev.rs`**: 
   - Add `/watch all` subcommand that auto-detects both lint and test commands using existing `lint_command_for_project()` and `test_command_for_project()`, then chains them with `&&`
   - Update `WATCH_SUBCOMMANDS` to include `"all"`
   - Update the `handle_watch` function to handle the `"all"` case
   - When auto-watch triggers (from Task 1), if both lint and test are detected, use the combined command

### Example behavior

```
/watch all
👀 Watch mode ON — will run `cargo clippy -- -D warnings && cargo test` after agent edits
```

```
/watch cargo test
👀 Watch mode ON — will run `cargo test` after agent edits
```

### Testing

- Test that `handle_watch` with `"all"` input in a Rust project directory produces a combined lint+test command
- Test that the combined command string contains both lint and test components
- Test WATCH_SUBCOMMANDS includes "all"

### Why this matters

This closes the gap with Aider's lint+test loop. A single `/watch all` (or auto-watch from Task 1 if both are available) ensures both code quality and correctness are verified after every agent edit, not just one or the other.
