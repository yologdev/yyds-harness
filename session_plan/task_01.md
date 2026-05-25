Title: Source context injection in watch-mode fix prompts
Files: src/watch.rs
Issue: none

## What

When watch mode detects build/test failures and constructs a fix prompt via `build_watch_fix_prompt`, it currently sends only the raw error output and structured error summaries. The fixing agent must then use `read_file` tool calls to see the actual source code before it can fix anything — wasting a full turn.

## Why

This is the single biggest leverage point for fix-loop efficiency. Commercial agents (Claude Code, Cursor) succeed on first fix attempts more often because they present the relevant code alongside the error. Our structured error parser (`parse_rust_errors`, `parse_typescript_errors`, `parse_python_errors`) already extracts file paths and line numbers from errors — we just need to read those files and inject the relevant lines.

## Implementation

1. **Add a helper function** `fn extract_error_source_context(errors: &[CompilerError]) -> String` in `src/watch.rs`:
   - Deduplicate file paths from the error list
   - For each unique `(file, line)`, read the file and extract lines `max(1, line-5)..=line+5` (11-line window)
   - If the file doesn't exist or can't be read, skip silently
   - Cap total injected context at ~3KB (roughly 50 lines across all files) to avoid bloating the prompt
   - Format as a markdown section with file headers and line numbers

2. **Modify `build_watch_fix_prompt`** to call the helper after building the error summary, and append the source context section to the prompt if non-empty. The section should be clearly labeled, e.g.:
   ```
   ## Relevant source context
   
   **src/foo.rs** (around line 42):
   ```rust
   37: fn example() {
   38:     let x = 5;
   ...
   ```

3. **Add tests**:
   - Test `extract_error_source_context` with mock CompilerError structs (file paths that don't exist → empty result)
   - Test that the function respects the 3KB cap
   - Test deduplication (same file mentioned in multiple errors → only included once)
   - Test that `build_watch_fix_prompt` includes source context when errors have file references

## Constraints
- Only touch `src/watch.rs`
- Don't change the CompilerError struct — just read its existing `file` and `line` fields
- Keep the context window reasonable — max ~3KB of injected source
- Files that don't exist or fail to read should be silently skipped (the agent is in a temp dir or the file was deleted)
