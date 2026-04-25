Title: Show custom commands in /help and support /help <custom-cmd>
Files: src/help.rs
Issue: none

The custom slash commands feature (`.yoyo/commands/` and `~/.yoyo/commands/`) is fully
implemented in `commands.rs` (discover, load, dispatch) and wired into the REPL
(tab completion, hinting) and `dispatch.rs` (execution). But `/help` doesn't show
any custom commands — a user who creates custom commands has no way to discover them
through the help system.

**What to implement:**

1. At the end of `help_text()` in `help.rs`, after the "Input" section, add a dynamic
   "Custom" section that calls `crate::commands::discover_custom_commands()` and lists
   any found commands with their descriptions (first line of the .md file).
   Only show the section if at least one custom command exists.

   Format:
   ```
     ── Custom ──
     /my-cmd          First line of my-cmd.md
     /deploy          First line of deploy.md
   ```

2. In `handle_help_command()`, add support for custom command names. If the input
   doesn't match any built-in command, check `crate::commands::get_custom_command_content(cmd)`
   and if found, display the content of the .md file as the help text.

3. In `help_command_completions()`, append custom command names to the completion list
   so `/help <Tab>` includes custom commands.

4. Add tests:
   - Test that `help_text()` includes a "Custom" section when custom commands are present
     (this may need a mock or temp dir approach similar to tests in commands.rs)
   - Test that `help_command_completions` returns custom commands

**Important:** The `discover_custom_commands()` function returns `Vec<(String, String)>`
where each tuple is `(name, description)`. Use this directly.
