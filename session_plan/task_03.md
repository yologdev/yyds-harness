Title: Harden byte-indexing in commands_git_review.rs and commands_move.rs with safety comments and guards
Files: src/commands_git_review.rs, src/commands_move.rs
Issue: none

## What

The assessment identified 63 byte-indexing sites across the codebase without `is_char_boundary()` guards. This task tackles the two command files with user-facing text processing: `commands_git_review.rs` (6 sites) and `commands_move.rs` (4 sites).

These files handle user-provided file paths (blame args), user-provided method names (move args), and git blame output — all of which can contain multi-byte UTF-8 characters.

## Implementation

### commands_git_review.rs (~6 sites)

1. **`parse_blame_args` (line ~359):** `&arg[..colon_pos]` — colon_pos from `rfind(':')`, always a char boundary since `:` is ASCII. Add safety comment.

2. **`parse_blame_args` range parsing (lines ~362-365):** `&range_part[..dash_pos]` and `&range_part[dash_pos + 1..]` — dash_pos from `find('-')`, ASCII safe. Add safety comment.

3. **`colorize_blame_line` (line ~451):** `&date_and_lineno[..last_space]` — last_space from `rfind(' ')`, ASCII safe. Add safety comment.

4. **Any other arithmetic-based indexing:** Add `is_char_boundary()` guard.

### commands_move.rs (~4 sites)

1. **`parse_move_args` (line ~80):** `&trimmed[..pos]` — pos from `find("::")`, ASCII safe. Add safety comment.

2. **Line ~317:** `&sample_line[..indent_len]` — indent_len computed from `len() - trimmed.len()`. This IS potentially dangerous if the line starts with multi-byte chars, but `trim_start()` removes ASCII whitespace, so the difference is the length of leading whitespace (ASCII). Safe, but add comment explaining why.

3. **Other sites:** Document safety reasoning.

### Tests
- Add test for `parse_blame_args` with a file path containing unicode characters (e.g., `/src/données.rs:10-20`).
- Add test for `parse_move_args` with a method name containing unicode (e.g., `Type::método`).
- Verify existing tests still pass.

## Sizing
2 files, mostly adding safety comments and a few tests. Well within 20 minutes.
