Title: Real-time bash output streaming via on_progress
Files: src/tools.rs
Issue: none

## Goal

Close the #1 gap vs Claude Code: real-time subprocess output streaming. When the bash tool
runs a command (especially long-running ones like `cargo test` or `npm install`), each line
of stdout/stderr should appear in the terminal as it arrives, not after the command finishes.

## What to change

In `StreamingBashTool::execute` (src/tools.rs), the reader loop already reads stdout/stderr
line-by-line. Currently it accumulates lines into a buffer and emits periodic
`ToolExecutionUpdate` events (via `emit_update` → `ctx.on_update`) containing the full
accumulated output. The renderer shows these as a line count + partial tail.

**Add real-time line emission using `ctx.on_progress`:**

1. In the reader loop, after each line is read and appended to the accumulator, call
   `ctx.on_progress` (if present) with the new line. This emits an
   `AgentEvent::ProgressMessage` which the prompt.rs renderer already handles — it prints
   each message as `{DIM}  {text}{RESET}`.

2. The `on_progress` call should happen for EVERY new line, not just on the update interval.
   Keep the existing `emit_update` (on_update) calls for backward compatibility — they still
   serve the purpose of giving the agent partial results for tool output compression.

3. Add a prefix to distinguish stdout vs stderr lines in the progress output:
   - stdout lines: emit as-is (they're the main output)
   - stderr lines: prefix with a subtle marker like `stderr: ` so the user can tell them apart

4. Only emit via on_progress when stderr is a terminal (i.e., interactive mode). In piped
   mode, the streaming output would interfere with structured output. Check
   `crate::format::stderr_is_terminal()` — if false, skip the on_progress calls.

## What NOT to change

- Don't change the final ToolResult format — the agent still gets the full buffered output
- Don't change emit_update behavior — it still provides periodic accumulated snapshots
- Don't change format/tools.rs or prompt.rs — they already handle ProgressMessage correctly
- Don't change the update_interval or lines_per_update fields

## Testing

Add tests that verify:
- `on_progress` is called with each line of output from a simple command
- stderr lines are prefixed appropriately
- The final result still contains the complete accumulated output
- Cancellation and timeout still work correctly

## Why this matters

This has been gap #1 since Day 38. Every competitor (Claude Code, Codex, Aider) shows
subprocess output in real time. A developer running `cargo test` through yoyo currently
sees nothing until the test suite finishes, which can be minutes for large projects. After
this change, they'll see each test result as it happens.
