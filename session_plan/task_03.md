Title: Add failure context to /retry — include last tool error and recovery guidance
Files: src/commands_retry.rs, src/prompt_retry.rs
Issue: none

## Context

Currently `/retry` re-sends the last user prompt when the agent gets stuck. But it doesn't include the specific tool error that caused the failure, which means the agent often repeats the same mistake. The `tool_recovery_hint()` function in `prompt_retry.rs` already maps tool names to recovery advice, but this knowledge isn't surfaced during `/retry`.

This improves the "graceful degradation" competitive gap (Priority Queue #2) by making recovery smarter, not just retries of the same approach.

## What to do

### In `src/commands_retry.rs`:

1. Modify `handle_retry()` to accept an optional `last_tool_error: &Option<String>` parameter (or read it from whatever state tracks the last error). When `/retry` fires AND a tool error is available, prepend context to the retry prompt:
   ```
   "The previous attempt failed with this tool error: {error}. 
    Try a different approach — {recovery_hint}."
   ```

2. Use `crate::prompt_retry::tool_recovery_hint()` to generate the recovery advice based on the tool name extracted from the error. If the tool name can't be determined from the error string, fall back to a generic message.

3. Add a helper function `fn extract_tool_name_from_error(error: &str) -> Option<&str>` that parses tool error messages to find the tool name. Tool errors typically contain patterns like `"Tool 'edit_file' failed"` or similar — check the actual error format from yoagent's `ToolError` type.

### In `src/prompt_retry.rs`:

4. If `tool_recovery_hint()` is currently `pub(crate)`, verify it's accessible from `commands_retry.rs` (it should be since both are in the same crate). No changes needed if already `pub(crate)` or `pub`.

### Tests (in `src/commands_retry.rs`):

5. Add tests for `extract_tool_name_from_error`:
   - Input containing `"edit_file"` → returns `Some("edit_file")`
   - Input containing `"bash"` → returns `Some("bash")`
   - Input with no recognizable tool name → returns `None`
   - Empty input → returns `None`

6. Add tests verifying that when tool error context is available, the retry prompt is enriched (test the prompt construction, not the actual retry execution).

## Key constraint

- Do NOT change the signature of `handle_retry` in a breaking way — the caller in `repl.rs` passes arguments positionally. If adding a parameter, make sure to update the call site in `repl.rs` as well. BUT if this would touch a 3rd file, instead store the last tool error in an accessible location (like an `Option<String>` that's already threaded through, or `last_error` which already exists as a parameter).
- Actually, `handle_retry` already takes `last_error: &Option<String>` — this likely already contains the error text. Check whether it contains tool-specific error information. If it does, just parse it; if it doesn't, document what it contains so a future task can wire it properly.

## Verification

```bash
cargo test commands_retry -- --nocapture
cargo clippy --all-targets -- -D warnings
```
