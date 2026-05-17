Title: Permission persistence — offer to save "always" approvals to .yoyo.toml
Files: src/tools.rs, src/config.rs
Issue: none

## What to do

Close the "smart permission cycle" competitive gap vs Claude Code. Currently when a user says "a" (always) at the bash confirmation prompt, it sets a session-only flag. The approval is forgotten when the session ends. Claude Code has `permission-mode` where approvals can be remembered across sessions.

**Implementation:**

1. In `src/config.rs`, add a function `append_allow_pattern(pattern: &str)` that:
   - Reads the `.yoyo.toml` file (or creates it if missing)
   - Appends the pattern to the `[permissions]` section's `allow` array
   - Writes the file back
   - Use the existing `write_config_value_to` / `set_toml_key` helpers if applicable, or write a focused helper

2. In `src/tools.rs`, modify the bash confirmation flow (around line 720-727, where "always" is handled):
   - After the user says "a"/"always", print the current auto-approved message
   - Then ask: "Save 'cargo test' to .yoyo.toml allow list? (y/n)"
   - Use a simplified version of the command as the pattern (strip arguments that look like paths/values, keep the base command + subcommand)
   - If they say yes, call `append_allow_pattern` and print confirmation
   - If they say no, just continue with session-only approval as before

3. Add a helper function `simplify_command_pattern(cmd: &str) -> String` in `src/tools.rs` that extracts a reasonable glob pattern from a bash command:
   - `cargo test` → `cargo test*`
   - `cargo build --release` → `cargo build*`
   - `npm run test` → `npm run test*`
   - `git commit -m "whatever"` → `git commit*`
   - Basic heuristic: keep first 2-3 words, append `*`

**Tests to add:**
- Test `simplify_command_pattern` with various commands
- Test `append_allow_pattern` creates/updates .yoyo.toml correctly (use temp dir)
- Test that the pattern actually matches via `PermissionConfig::check`

**Key constraint:** The "save to toml" prompt should only fire once per session for the same base pattern — don't re-ask if they already declined saving a `cargo*` pattern.

**Don't change:** The existing "always" session-only behavior should still work exactly as before. The save-to-toml prompt is an optional follow-up, not a replacement.
