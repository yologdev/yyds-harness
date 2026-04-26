Title: Auto-enable watch mode on session start when a project type is detected
Files: src/repl.rs, src/commands_dev.rs, src/config.rs
Issue: none

## What to do

This is the single biggest behavioral gap between yoyo and Aider. Aider automatically runs lint+test after every agent edit and auto-fixes failures. yoyo already has `/watch` which does exactly this — but the user has to manually type `/watch` first. Most users never discover it.

### Implementation

1. **In `src/commands_dev.rs`**: Add a new public function `auto_detect_watch_command()` that uses the existing `detect_test_command()` (which already uses `detect_project_type()`) to find the appropriate test command for the current project. Return `Option<String>`.

2. **In `src/config.rs`**: Add a new config key `auto_watch` to `SETTABLE_KEYS` (boolean, default `true`). Add `validate_config_value` support. Add a parser function `parse_auto_watch_from_config(config: &toml::Value) -> bool` that reads `auto_watch` from the TOML config. Default is `true` — watch mode is on by default for detected projects.

3. **In `src/repl.rs`**: At the start of `run_repl()`, after the welcome message is printed but before the main loop, check if:
   - Watch mode is not already active (no `/watch` command set)
   - `auto_watch` config is enabled (default: true)
   - A test command can be auto-detected
   
   If all three are true, call `set_watch_command()` with the detected command and print a dim message:
   ```
   👀 Auto-watch: `cargo test` (disable with /watch off or auto_watch = false)
   ```

### Testing

- Add a test in `commands_dev.rs` that `auto_detect_watch_command()` returns `Some("cargo test ...")` when run from a directory with `Cargo.toml`
- Add a test that the config parsing correctly reads `auto_watch = false`
- Add a test that `/watch off` clears the watch command (already exists, verify it still works)

### Config documentation

Users can disable in `.yoyo.toml`:
```toml
auto_watch = false
```

Or mid-session:
```
/config set auto_watch false
```

Or for one session:
```
/watch off
```

### Why this matters

This turns yoyo's existing edit→test→fix loop from opt-in to default. A developer who opens yoyo in a Rust/Node/Python/Go project immediately gets Aider-style automatic test verification after every agent edit, without knowing about `/watch`. This is the single biggest competitive gap in the interactive experience.
