Title: Add argument-position hints for slash commands
Files: src/repl.rs, src/commands.rs
Issue: #214

## What

When a user types `/diff ` (command + space), the inline hint currently disappears because the hinter only handles command-name completion. Add argument-position hinting so users see available subcommands/flags after typing a command name.

Examples of what the user should see (in dim text, like existing hints):
- `/diff ` → hint: `[file] [--stat] [--cached] [--staged] [--name-only]`
- `/model ` → hint: `<model-name>`  
- `/think ` → hint: `off | low | medium | high`
- `/git ` → hint: `status | log | stash | branch | ...`
- `/pr ` → hint: `create | describe | status | diff`
- `/save ` → hint: `<filename.json>`
- `/help ` → hint: `<command>`
- `/config ` → hint: `show | edit | hooks | permissions | mcp | teach`

## How

1. In `src/commands.rs`, add a new function `pub fn command_arg_hint(cmd: &str) -> Option<&'static str>` that returns a hint string for each command that accepts arguments. Use the existing `*_SUBCOMMANDS` constants and `DIFF_FLAGS` to build these. Commands with no arguments return `None`.

2. In `src/repl.rs`, modify the `Hinter::hint()` implementation for `YoyoHelper`:
   - Currently, when `typed.contains(' ')` it returns `None`
   - Instead, when the user has typed a command + space but no argument yet (or partial argument), show the argument hint
   - Specifically: if `typed.contains(' ')`, split on first space to get `(cmd, arg_part)`. If `arg_part` is empty, call `command_arg_hint` and return the hint. If `arg_part` is non-empty, keep returning `None` (they're already typing an argument, tab-completion handles the rest).

3. Add tests:
   - Test that `command_arg_hint("/diff")` returns something containing "--stat"
   - Test that `command_arg_hint("/help")` returns something containing "command"
   - Test that `command_arg_hint("/version")` returns `None` (no arguments)
   - Test the hinter returns argument hints when line is "/diff " (with trailing space)

## Verification

- `cargo build && cargo test` passes
- `cargo clippy --all-targets -- -D warnings` passes
- The hint system is purely visual — no functional behavior changes, low regression risk
