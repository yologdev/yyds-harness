Title: Add regex-error recovery hint to search tool error messages
Files: src/tools.rs
Issue: none
Origin: planner

Objective:
Reduce `search_regex_error` count (57 occurrences in trajectory) by adding a recovery hint to the search tool's error message when a regex parse failure is detected, guiding the agent to retry with `regex=false`.

Why this matters:
`search_regex_error` is the single most frequent tool failure category (57 occurrences). These failures waste turns during assessment and implementation phases. The search tool already defaults to literal (`regex=false`) search, but when the agent explicitly passes `regex=true` with unescaped metacharacters, the error message gives no guidance on how to recover. A one-line hint in the error message can prevent the agent from retrying the same broken pattern.

Success Criteria:
- When ripgrep/grep exits with code 2 and stderr contains regex parse error keywords ("unmatched", "invalid", "regex parse", "unclosed"), the error message includes a hint like "Hint: try regex=false for literal search or escape regex metacharacters."
- Normal search errors (file not found, permission denied) are unchanged.
- Existing tests pass.

Verification:
- `cargo test --lib` — search tool tests
- `cargo build` passes
- Manual: search for `(` with `regex=true` and verify the error message includes a recovery hint.

Expected Evidence:
- `search_regex_error` count decreases in future trajectory tool-failure summaries.
- Agent retries after regex errors are more likely to succeed.

Implementation Notes:
- In `src/tools.rs`, around line 243-248, the error handler for non-zero/non-one exit codes:
  ```rust
  if code != Some(0) && code != Some(1) {
      return Err(ToolError::Failed(format!(
          "Search error: {}",
          stderr.trim()
      )));
  }
  ```
- Add regex error detection: check if stderr contains any of: "unmatched", "invalid", "regex parse", "unclosed", "empty pattern", "repetition".
- If detected AND `regex` is true, append: " Hint: try regex=false for literal search, or escape regex metacharacters with \\."
- Keep the change minimal — just the error message enhancement, no retry logic.
- Add a test in the existing search tool test module that verifies the hint appears in the error message for a regex error scenario.
