Title: Add tests for format/output.rs compression and truncation logic
Files: src/format/output.rs
Issue: none

## Description

`format/output.rs` (1,683 lines, 68 tests) contains critical logic for tool output compression, filtering, truncation, and formatting. The test-to-line ratio is 1:24 which is adequate but the most important functions deserve deeper edge-case coverage:

### Functions to add tests for

1. **`compress_tool_output`** — the main compression pipeline. Add tests for:
   - Empty input returns empty output
   - Short input (under threshold) is returned unchanged
   - Repeated blank lines are collapsed
   - Consecutive duplicate lines are collapsed with count
   - Very long lines are truncated
   - Mixed content with some repeated sections and some unique content

2. **`filter_test_output`** — filters noisy test output. Add tests for:
   - Rust `cargo test` output filtering (keeps failures, drops passing)
   - Input with no test markers is returned unchanged
   - Edge case: only pass lines, no fails → compressed output

3. **`smart_truncate_for_context`** — intelligent truncation for context injection. Add tests for:
   - Content under limit is returned unchanged
   - Content over limit is truncated with indicator
   - Multi-section content keeps headers and truncates body
   - Empty content
   - Content exactly at limit

4. **`truncate_tool_output`** — basic truncation with byte limit. Add tests for:
   - Multi-byte UTF-8 characters don't cause panics (critical safety — see Safety Rules in CLAUDE.md)
   - Content under limit passes through
   - Content at limit boundary

5. **`format_tool_batch_summary`** — batch summary formatting. Add tests for:
   - Empty tool list
   - Single tool
   - Multiple tools with varying result sizes

### Test approach

All functions are pure (take input, return output) so tests are straightforward unit tests. Add them in the existing `#[cfg(test)] mod tests` block at the bottom of the file.

Target: add 12-15 new test functions covering the edge cases above. Focus especially on UTF-8 safety (multi-byte chars at truncation boundaries) since the Safety Rules call this out as a past production crash.

### Verification
`cargo build && cargo test && cargo clippy --all-targets -- -D warnings`
