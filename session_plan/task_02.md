Title: Show colored diff preview in edit_file confirmation prompt
Files: src/tool_wrappers.rs, src/format/diff.rs
Issue: none

## Context

When yoyo asks for permission to edit a file (`edit_file`), the confirmation prompt shows only a summary like `edit: foo.rs (3 → 5 lines)`. The user has to approve the change without seeing WHAT's actually changing. Claude Code shows the actual diff before the edit is applied. This is a trust and transparency gap.

Day 73 added colored diffs for `write_file` — showing old vs new content when overwriting a file. The same treatment should be applied to `edit_file` confirmations, which is arguably more important since edits are more surgical and the user needs to verify the right text is being matched.

## What to do

1. **In `tool_wrappers.rs`, enhance the `ConfirmTool` execution for `edit_file`** to show a colored inline diff of old_text vs new_text before asking for confirmation. The diff should use the existing `format::diff` module (which already has LCS-based line diff and colored rendering).

2. **The flow should be:**
   ```
   ✎ edit: src/main.rs (3 → 5 lines)
   
   - old line that's being removed
   + new line that's being added
   + another new line
   
   (y)es / (n)o / (a)lways / (d)iff:
   ```

3. **Implementation approach:**
   - In the `ConfirmTool::call` method (or in `confirm_file_operation`), when the tool is `edit_file`, extract `old_text` and `new_text` from the params
   - Use `format::diff::line_diff` (or the appropriate function from `format/diff.rs`) to generate a colored diff
   - Print the diff to stderr before the confirmation prompt
   - Keep the diff compact: if old_text + new_text > 40 lines combined, show only the first/last 10 lines with a `... (N more lines) ...` ellipsis

4. **Write tests** for the describe function to verify the diff is generated correctly for various edit_file param combinations.

## What NOT to do

- Don't change the behavior when `always_approved` is true — auto-approved edits should still show the existing one-line summary, not a full diff
- Don't change `write_file` behavior (it already has its own diff display from Day 73)
- Don't touch `describe_file_operation` — the summary line stays the same; the diff is additional output

## Verification

- `cargo build` passes
- `cargo test` passes (including new tests)
- `cargo clippy --all-targets -- -D warnings` passes
