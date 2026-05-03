Title: Extract message search and utility functions from prompt.rs into prompt_utils.rs
Files: src/prompt_utils.rs (new), src/prompt.rs
Issue: none

## Motivation

After Task 2 extracts retry/error logic, prompt.rs still has a second cohesive group of
utility functions unrelated to the core prompt execution loop: message searching, text
highlighting, message summarization, and output file writing. These are ~170 lines of
functions + ~250 lines of tests. Extracting them further reduces prompt.rs and creates
a focused utilities module.

## What to extract into `src/prompt_utils.rs`

### Functions:
- `write_output_file` — writes text to an optional output file path
- `message_text` — extracts plain text from an AgentMessage
- `tool_result_preview` — formats a preview of a ToolResult
- `highlight_matches` — highlights search query matches in text
- `search_messages` — searches through conversation messages
- `summarize_message` — produces a one-line summary of a message

### Tests to move:
- `test_summarize_message_user`
- `test_summarize_message_tool_result`
- `test_summarize_message_tool_result_error`
- `test_write_output_file_none`
- `test_write_output_file_some`
- `test_tool_result_preview_empty`
- `test_tool_result_preview_text`
- `test_tool_result_preview_truncated`
- `test_tool_result_preview_multiline`
- `test_search_messages_basic_match`
- `test_search_messages_case_insensitive`
- `test_search_messages_no_match`
- `test_search_messages_empty_messages`
- `test_search_messages_multiple_matches`
- `test_search_messages_tool_result`
- `test_message_text_user`
- `test_message_text_tool_result`
- `test_highlight_matches_basic`
- `test_highlight_matches_case_insensitive`
- `test_highlight_matches_multiple_occurrences`
- `test_highlight_matches_no_match`
- `test_highlight_matches_empty_query`
- `test_highlight_matches_empty_text`
- `test_highlight_matches_preserves_original_case`

## How to do it

1. Create `src/prompt_utils.rs` with the extracted functions and their tests
2. In `prompt.rs`, replace the extracted code with imports from the new module
3. Add `mod prompt_utils;` to `main.rs`
4. Make sure visibility is correct — `search_messages`, `highlight_matches`, etc. are used
   by `commands_session.rs` (for `/search`) so they need to be `pub(crate)` or `pub`
5. Check all call sites: `prompt.rs` itself, `commands_session.rs`, `repl.rs`, `dispatch.rs`
   — anywhere these utilities are referenced via `crate::prompt::`

## Important dependency note

This task MUST run AFTER Task 2 (prompt_retry.rs extraction). If Task 2 hasn't been applied
yet, the line numbers in prompt.rs will be wrong. The implementation agent should verify
prompt.rs's current state before making edits.

## Verification

- `cargo build` — compiles
- `cargo test` — all tests pass
- `cargo clippy --all-targets -- -D warnings` — clean
- Verify prompt.rs is now significantly smaller (target: under 1,800 lines)

## Constraints
- Do NOT change any function signatures or behavior — pure extraction
- Do NOT rename anything
- Move tests alongside their functions
