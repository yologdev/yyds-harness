Title: Custom slash commands — load user-defined commands from .yoyo/commands/
Files: src/dispatch.rs, src/commands.rs, src/repl.rs
Issue: none

## What

Claude Code supports custom slash commands loaded from `.claude/commands/`. This is a listed capability gap. Implement a minimal version: load markdown files from `.yoyo/commands/` (project-local) and `~/.yoyo/commands/` (global) and register them as slash commands in the REPL.

When a user creates `.yoyo/commands/review.md` with content like:
```
Review the current git diff and suggest improvements. Focus on:
- Code quality
- Potential bugs
- Missing tests
```

They can type `/review` in the REPL and the file contents are sent as the user message to the agent.

## Implementation

### 1. In `src/commands.rs`: Add custom command discovery

Add a function `discover_custom_commands() -> Vec<(String, String)>` that:
- Scans `.yoyo/commands/*.md` (project-local, higher priority)
- Scans `~/.yoyo/commands/*.md` (global/user-level)  
- For each `.md` file, extracts the stem as the command name and reads the file content
- Returns `Vec<(name, content)>` — deduplicates by name (project-local wins over global)
- Silently returns empty vec if directories don't exist

Also add `is_custom_command(cmd: &str) -> bool` and `get_custom_command_content(cmd: &str) -> Option<String>` helpers.

### 2. In `src/dispatch.rs`: Handle custom commands in dispatch

In `dispatch_command()`, after checking all known built-in commands but before returning "unknown command", check if the input matches a custom command:

```rust
// Check custom commands
let cmd_name = /* extract first word from input */;
if let Some(content) = commands::get_custom_command_content(cmd_name) {
    // Return the content as a prompt to send to the agent
    return Ok(CommandResult::Prompt(content));
}
```

Add a `Prompt(String)` variant to `CommandResult` if one doesn't exist. The REPL will handle this by sending the content as a user message to the agent.

### 3. In `src/repl.rs`: Handle the Prompt result and add completions

In the REPL's command handling, when `CommandResult::Prompt(content)` is returned, send the content as the next user message to the agent (similar to how regular user input is handled).

Add custom command names to the tab-completion list in `YoyoHelper`. The `complete` method already has access to known commands — add discovered custom commands to that list.

### Tests

Add tests in `src/commands.rs`:
- `test_discover_custom_commands_empty` — no `.yoyo/commands/` dir returns empty vec
- `test_discover_custom_commands_finds_files` — create temp dir with `.yoyo/commands/test.md`, verify discovery
- `test_custom_command_project_overrides_global` — project-local command with same name wins

## Verification

- `cargo build && cargo test`
- Manual: create `.yoyo/commands/hello.md` with "Say hello!", type `/hello` in REPL → agent receives "Say hello!" as prompt
