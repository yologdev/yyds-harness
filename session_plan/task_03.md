Title: Expand smart_edit.rs test coverage
Files: src/smart_edit.rs
Issue: none

`smart_edit.rs` has only 10 tests for 758 lines of code — the thinnest coverage ratio of any
file in the codebase (75 lines/test). This is core infrastructure: the SmartEditTool wraps
edit_file with fuzzy matching and whitespace auto-fix. Gaps in test coverage here mean gaps
in the edit experience.

## What to test

### `find_nearest_match()` edge cases (add ~8 tests)

Current tests cover: exact line match, whitespace-only diff, no match. Missing:
- Multi-line old_text where the best match has extra/fewer blank lines
- old_text that matches at the very start of the file (line 1)
- old_text that matches at the very end of the file (last line, no trailing newline)
- Very short old_text (1-2 chars) — should NOT match too aggressively
- old_text with mixed indentation (tabs vs spaces) vs file with consistent spaces
- File with multiple partial matches — should find the NEAREST/best one
- Unicode content in both file and old_text
- Empty file content → should return None

### `augment_not_found_error()` edge cases (add ~4 tests)

Current tests may not cover:
- Error message when the file doesn't exist (params with bad path)
- Error message when old_text is very long (should show truncated snippet in hint)
- Error with params missing required fields
- Line number hint accuracy — verify the reported line number is correct

### `try_whitespace_autofix()` behavior (add ~3 tests)

This is an async function but test the logic paths:
- Whitespace-only diff detected → should attempt auto-fix
- Non-whitespace diff → should NOT attempt auto-fix
- The `is_whitespace_only` flag from `find_nearest_match` controls this path

Focus on unit-testable functions (`find_nearest_match`, `augment_not_found_error`).
The async `try_whitespace_autofix` and `execute` methods can be tested via their
pure-logic subcomponents rather than full async execution.

Verify with `cargo test smart_edit` after adding tests.
