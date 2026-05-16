Title: Add unit tests for tool_wrappers.rs — ToolFailureTracker and truncate_result
Files: src/tool_wrappers.rs
Issue: none

## What to do

Add unit tests for the untested parts of `tool_wrappers.rs`, which has the worst test coverage ratio of important files at 60 lines/test (1,520 lines / 25 tests). The existing tests cover `describe_file_operation` and `format_edit_diff_preview` well, but `ToolFailureTracker`, `truncate_result`, and `with_recovery_hints` have zero tests.

### Tests to add

**ToolFailureTracker tests** (pure logic, no async needed):
1. `test_tracker_new_is_empty` — new tracker has 0 count for any tool name.
2. `test_tracker_record_failure_increments` — calling `record_failure("bash")` 3 times returns 1, 2, 3.
3. `test_tracker_record_success_resets` — after 3 failures, `record_success` resets count to 0.
4. `test_tracker_independent_tools` — failures for "bash" don't affect count for "edit_file".
5. `test_tracker_clone_shares_state` — cloned tracker shares the same Arc<Mutex<HashMap>>.

**truncate_result tests:**
6. `test_truncate_result_short_text_unchanged` — text under max_chars passes through unchanged.
7. `test_truncate_result_long_text_truncated` — text over max_chars gets truncated (check it's shorter after).
8. `test_truncate_result_non_text_content_unchanged` — non-Text Content variants pass through unchanged.
9. `test_truncate_result_empty_content` — empty content vec stays empty.

**describe_file_operation edge cases** (supplement existing tests):
10. `test_describe_read_file_operation` — read_file should return appropriate description.
11. `test_describe_bash_operation` — bash tool description.

### Implementation notes
- `ToolFailureTracker` already has `#[cfg(test)] fn get()` — use it.
- `truncate_result` takes a `yoagent::types::ToolResult` — construct one with `ToolResult { content: vec![Content::Text { text: ... }] }`. Check yoagent's `ToolResult` struct for the exact fields.
- All tests are synchronous (no async needed for tracker or truncate_result).
- Add tests inside the existing `#[cfg(test)] mod tests` block at the bottom of the file.

### Verify
`cargo build && cargo test`
