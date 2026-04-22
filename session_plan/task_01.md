Title: Extract tool output compression/filtering into format/output.rs
Files: src/format/mod.rs, src/format/output.rs
Issue: none

## What to do

Extract the tool output compression, filtering, and truncation code from `src/format/mod.rs` into a new `src/format/output.rs` module. This reduces mod.rs from ~3,092 lines toward ~2,000 lines and creates a focused module for output processing.

## What to move

Move these items from `format/mod.rs` to `format/output.rs`:

**Constants** (lines ~176-193):
- `TOOL_OUTPUT_MAX_CHARS`
- `TOOL_OUTPUT_MAX_CHARS_PIPED`
- `TRUNCATION_HEAD_LINES`
- `TRUNCATION_TAIL_LINES`
- `COLLAPSE_MIN_LINES`
- `CATEGORY_PREFIX_MAX`
- `TEST_FILTER_MIN_PASS_LINES`

**Public functions**:
- `compress_tool_output`
- `filter_test_output`
- `truncate_tool_output`
- `format_tool_batch_summary`
- `indent_tool_output`

**Private helpers** (used only by the above):
- `strip_ansi_codes`
- `is_progress_bar_line`
- `is_compiling_line`
- `is_downloading_line`
- `filter_noisy_patterns`
- `line_category`
- `collapse_repetitive_lines`
- `TestLineKind` enum
- `classify_test_line`
- `terminal_width` (if used only by output functions; if shared, keep in mod.rs)

**All tests** for the above functions from the `mod tests` block at the bottom of mod.rs.

## How to do it

1. Create `src/format/output.rs`
2. Add `mod output;` and `pub use output::*;` in `format/mod.rs` (alongside existing `mod cost;`, `mod highlight;`, etc.)
3. Move the listed items into `output.rs`
4. Add necessary `use` imports in `output.rs` (it may need `super::safe_truncate` or similar utilities from mod.rs)
5. Remove the moved code from `mod.rs`
6. Move all associated tests from the `#[cfg(test)] mod tests` block to a `#[cfg(test)] mod tests` block in `output.rs`
7. Run `cargo build && cargo test` to verify everything compiles and all tests pass
8. Run `cargo clippy --all-targets -- -D warnings` to verify no warnings
9. Run `cargo fmt` to auto-format

## Key considerations

- `terminal_width()` may be used by both output.rs functions and other mod.rs functions. Check its usage — if it's only used by output functions, move it. If shared, keep it in mod.rs and use `super::terminal_width` from output.rs, or make it `pub(crate)`.
- The `safe_truncate` function is used broadly — keep it in mod.rs.
- Make sure `TOOL_OUTPUT_MAX_CHARS` and `TOOL_OUTPUT_MAX_CHARS_PIPED` remain `pub` since they're used in `src/main.rs` and `src/tools.rs`.
- All `pub` items must stay accessible as `crate::format::compress_tool_output` etc. via the `pub use output::*;` re-export.

## Verification

```bash
cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt -- --check
```

No behavior changes — pure code organization refactor.
