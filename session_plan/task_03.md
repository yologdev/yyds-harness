Title: Extract diff rendering into format/diff.rs
Files: src/format/mod.rs, src/format/diff.rs
Issue: none

## What to do

Extract the diff rendering logic from `src/format/mod.rs` into a new `src/format/diff.rs` module. This is the second extraction from mod.rs (after task 01 extracts the output filtering), further reducing it from ~2,000 lines (post-task-01) toward ~1,700 lines.

**Important**: This task runs AFTER task 01. Read the actual state of `format/mod.rs` as it exists when you start — task 01 may have already removed the output filtering code. If task 01 was reverted, the file will be at its original ~3,092 lines and you should still extract only the diff logic described below.

## What to move

Move these items from `format/mod.rs` to `format/diff.rs`:

**Constants**:
- `MAX_DIFF_LINES` (currently 20)
- `DIFF_CONTEXT_LINES` (currently 3)

**Types**:
- `DiffOp<'a>` enum (Equal, Insert, Delete variants)

**Functions**:
- `compute_line_diff` — LCS-based line diff algorithm
- `format_edit_diff` — renders old_text vs new_text as a colored unified diff

**All tests** for the above from the `mod tests` block.

## How to do it

1. Create `src/format/diff.rs`
2. Add `mod diff;` and `pub use diff::*;` in `format/mod.rs` (alongside existing module declarations)
3. Move the listed items into `diff.rs`
4. Add necessary `use` imports in `diff.rs`:
   - Will need `use super::{Color, RESET, RED, GREEN, DIM, CYAN};` or similar for ANSI color constants
   - May need `use super::safe_truncate;` if the diff code uses it
5. Remove the moved code from `mod.rs`
6. Move associated tests to `diff.rs`
7. Run full verification

## Key considerations

- `format_edit_diff` is called from `format/mod.rs`'s `format_tool_summary` function (for edit_file tool rendering). After extraction, `format_tool_summary` will call `diff::format_edit_diff` which is fine via the `pub use diff::*;` re-export.
- `DiffOp` is private (`enum`, not `pub enum`) — check if it needs to be `pub` or can stay `pub(super)`. Since it's only used by `compute_line_diff` and `format_edit_diff`, it can stay private within `diff.rs`.
- The `compute_line_diff` function is also private — keep it private within `diff.rs`.
- `format_edit_diff` is `pub` — keep it `pub`.
- This extraction is small (~250 lines of production code + tests) but creates a focused, coherent module.

## Verification

```bash
cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt -- --check
```

No behavior changes — pure code organization refactor.
