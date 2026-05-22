Title: Show compact colored diff in exit summary when quitting
Files: src/commands_retry.rs, src/repl.rs
Issue: none

When a user quits yoyo (Ctrl-D or /quit), the exit summary shows: duration, tokens, cost, and file names. But it doesn't show *what* changed — the user has to mentally reconstruct or run `/changes --diff` before quitting. Claude Code shows changes clearly at exit.

**What to build:**

Enhance `format_exit_summary` in `src/commands_retry.rs` to include a compact colored diff of session changes when files were modified. The diff should be truncated to at most 15 lines to keep the exit clean.

**Implementation approach:**

1. In `format_exit_summary()`, after the "Files changed" line, if there are modified files:
   - Call `collect_diffs(&paths)` (already exists in the same file) to get the raw git diff
   - Truncate the diff output to at most 15 lines (with a "... N more lines" note if truncated)
   - Append the truncated colored diff to the output lines

2. Add a helper `truncate_diff_lines(diff: &str, max_lines: usize) -> String` that:
   - Splits by newline, takes `max_lines` lines
   - If there were more, appends a dim "... and N more lines (use /changes --diff to see all)"
   - Preserves ANSI color codes in the truncated output

3. Guard: Don't show the diff if `is_quiet()` returns true (--print mode).

4. Guard: Only show the diff if the total diff is non-empty (committed changes won't show in `git diff`).

**In `src/repl.rs`:** No changes needed — `format_exit_summary` is already called at exit. The enhancement is purely within `commands_retry.rs`.

**Tests:**
- Test `truncate_diff_lines` with input shorter than max (returns unchanged)
- Test `truncate_diff_lines` with input longer than max (truncates with message)
- Test `truncate_diff_lines` with empty input (returns empty)
- Test that `format_exit_summary` includes diff content when changes exist (integration-style test with mocked git output — may need to just test the truncation helper since git calls are hard to mock)

**Constraints:**
- Keep the exit output clean — 15 lines max for the diff portion
- Use the existing `colorize_diff` function for coloring
- Don't break the existing exit summary format — add the diff *after* the existing lines
