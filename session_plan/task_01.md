Title: Extract session tracking types from prompt.rs into src/session.rs
Files: src/prompt.rs, src/session.rs, src/main.rs
Issue: none

## What

Extract the session-tracking data types and their implementations from `prompt.rs` (3,063 lines, second-largest file) into a new `src/session.rs` module. This continues the structural consolidation that successfully split `format/mod.rs` in Day 53.

## What to extract

The following types and their `impl` blocks, starting around line 169:

1. **`SessionChanges`** struct + both `impl` blocks (new, record, snapshot, clear, len, is_empty)
2. **`FileChange`** struct
3. **`ChangeKind`** enum + `Display` impl
4. **`TurnSnapshot`** struct + both `impl` blocks (new, snapshot_file, record_created, is_empty, restore, file_count)
5. **`TurnHistory`** struct + both `impl` blocks (new, push, len, is_empty, undo_last, clear, pop)
6. **`format_changes()`** function (at line ~1812)

Also extract their associated test modules (tests for SessionChanges, TurnSnapshot, TurnHistory, format_changes).

## How

1. Create `src/session.rs` with the extracted types
2. Add `pub mod session;` in `main.rs`
3. Update `prompt.rs` to `use crate::session::*;` (or specific imports) so all existing callers still work
4. The extracted module needs: `use std::collections::HashMap;`, `use std::path::Path;`, `use crate::format::{BOLD, CYAN, DIM, GREEN, RED, RESET, YELLOW};`
5. All `pub` visibility should be preserved
6. Run `cargo build && cargo test` to verify nothing breaks
7. Update CLAUDE.md's Architecture section to add `session.rs` with description

## Verification

- `cargo build` passes
- `cargo test` passes (all session tracking tests run from new location)
- `cargo clippy --all-targets -- -D warnings` passes
- `grep -c "SessionChanges\|TurnSnapshot\|TurnHistory" src/prompt.rs` should show only `use` imports, not definitions
