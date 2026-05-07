Title: Add AutoCheckTool wrapper for per-edit compilation feedback
Files: src/tool_wrappers.rs, src/tools.rs
Issue: none (addresses Gap #3: "Per-edit auto-lint-test" from CLAUDE_CODE_GAP.md)

## Motivation

This is the **#1 remaining competitive gap** vs Aider. Aider runs lint+test after each individual 
file write, catching errors before the agent moves on. yoyo's `/watch` only runs after the full 
prompt cycle. This means the agent can make 5 edits that each break the build, and only discover 
errors at the end.

By adding a lightweight wrapper that runs `cargo check` (or the configured watch command) after 
each write_file/edit_file, the agent gets immediate compilation feedback inline with the tool 
result — e.g., "File written successfully.\n\n⚠ Auto-check failed:\nerror[E0433]: ..."

## Implementation

### 1. Add `AutoCheckTool` wrapper in `src/tool_wrappers.rs`

```rust
/// A tool wrapper that automatically runs a check command after file edits.
/// When a watch command is configured, it runs the command after successful
/// write_file or edit_file operations and appends any errors to the tool result.
pub struct AutoCheckTool {
    inner: Box<dyn AgentTool>,
}
```

Key behaviors:
- Only activates when `watch::get_watch_command()` returns Some
- Only runs the check after SUCCESSFUL tool calls (don't double-report on already-failed edits)
- Appends check output to the tool result string (so the agent sees it immediately)
- Truncates check output to ~2000 chars to avoid flooding context
- Uses `watch::run_watch_command()` to run the check (reuses existing infrastructure)
- Does NOT block on long-running test suites — if the watch command includes tests, 
  consider using only the first command from `get_watch_commands()` (typically the lint phase)

Add a helper function:
```rust
pub fn with_auto_check(tool: Box<dyn AgentTool>) -> Box<dyn AgentTool> {
    Box::new(AutoCheckTool { inner: tool })
}
```

### 2. Wire into the tool chain in `src/tools.rs`

In `build_tools()`, wrap write_tool and edit_tool with `with_auto_check`:
```rust
// Current chain: confirm → guard → truncate → hook
// New chain:     confirm → guard → auto_check → truncate → hook
```

Place `auto_check` BEFORE truncation so the combined (original result + check output) gets
truncated together. Place it AFTER guard/confirm so checks only run on approved edits.

### 3. Tests

Add tests in `tool_wrappers.rs`:
- Test that AutoCheckTool passes through when no watch command is set
- Test that AutoCheckTool appends check output on failure
- Test truncation of long check output

## Scope limits
- Do NOT change the watch module itself
- Do NOT add new configuration — uses the existing `/watch set` mechanism
- This is purely a tool wrapper addition + wiring change
- The watch command must already be set (via `/watch set` or auto-detect) for this to activate
