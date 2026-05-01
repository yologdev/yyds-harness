Title: Enrich auto-retry with tool name and recovery hints
Files: src/prompt.rs
Issue: none

Improve the auto-retry system (gap #3: graceful degradation on tool failures) by tracking which tool failed and providing tool-specific recovery suggestions in the retry prompt.

### Current state:
- `PromptOutcome.last_tool_error` stores just the error text (a string)
- `build_auto_retry_prompt` says: "a tool failed with: {summary}. Try a different approach."
- The agent gets no information about WHICH tool failed, so it can't make informed retry decisions

### Changes to `src/prompt.rs`:

1. **Add `last_tool_name: Option<String>` to `PromptOutcome` struct** (line ~35):
   ```rust
   pub struct PromptOutcome {
       pub text: String,
       pub last_tool_error: Option<String>,
       pub last_tool_name: Option<String>,  // NEW
       pub was_overflow: bool,
       pub last_api_error: Option<String>,
   }
   ```

2. **Track tool_name alongside tool_error** in `handle_prompt_events` (~line 760-764):
   Where we set `last_tool_error = Some(error_text)`, also set a local `last_tool_name = Some(tool_name.clone())`.

3. **Pass `last_tool_name` through all PromptOutcome construction sites** — there are ~6 places where `PromptOutcome { ... }` is constructed. Add `last_tool_name` to each.

4. **Add `tool_recovery_hint(tool_name: &str) -> &str` helper function** that returns tool-specific recovery suggestions:
   - `"bash"` → "The shell command failed. Check if the command exists, try a simpler version, or use a different approach."
   - `"edit_file"` → "The edit failed (likely text mismatch). Use read_file to see current contents, then retry with exact text."
   - `"write_file"` → "The file write failed. Check the path exists and you have the right permissions."
   - `"read_file"` → "The file read failed. Use list_files to verify the path, or search for the file."
   - `"search"` → "The search failed. Try a simpler pattern or check the path."
   - `"rename_symbol"` → "The rename failed. Verify the symbol exists with search first."
   - default → "The tool call failed. Try a different approach."

5. **Update `build_auto_retry_prompt` signature** to accept an optional tool name:
   ```rust
   pub fn build_auto_retry_prompt(
       original_input: &str,
       tool_error: &str,
       tool_name: Option<&str>,
       attempt: u32,
   ) -> String
   ```
   Include the tool name and recovery hint in the retry prompt:
   ```
   [Auto-retry {attempt}/{MAX_AUTO_RETRIES}: {tool_name} failed with: {summary}. {recovery_hint}]
   ```

6. **Update the call site in `run_prompt_auto_retry`** (~line 1267) to pass `outcome.last_tool_name.as_deref()`.

7. **Add tests:**
   - `test_build_auto_retry_prompt_with_tool_name` — verify tool name appears in output
   - `test_build_auto_retry_prompt_without_tool_name` — verify graceful None handling
   - `test_tool_recovery_hint_bash` — verify bash-specific hint
   - `test_tool_recovery_hint_edit_file` — verify edit-specific hint
   - `test_tool_recovery_hint_unknown` — verify default hint

### Acceptance criteria:
- `PromptOutcome` has `last_tool_name: Option<String>`
- `build_auto_retry_prompt` includes tool name when available
- Tool-specific recovery hints exist for bash, edit_file, write_file, read_file, search, rename_symbol
- All existing tests pass
- New tests cover the added functionality
- `cargo build && cargo test && cargo clippy --all-targets -- -D warnings`
