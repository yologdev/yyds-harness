Title: Add unit tests for session.rs — SessionChanges, TurnSnapshot, TurnHistory, format_changes
Files: src/session.rs
Issue: none

## Goal

Strengthen test coverage for `session.rs`, which has 696 lines but only 29 tests. This module
handles session change tracking, turn snapshots for undo, and turn history — all critical for
the `/undo`, `/changes`, and session reliability. More tests here protect against regressions
in undo behavior and session state tracking.

## What to test

### SessionChanges (concurrent change tracking)
- `new()` creates empty state
- `record()` tracks file changes with correct ChangeKind
- `record()` for the same file multiple times (last-write wins in snapshot)
- `snapshot()` returns all recorded changes
- `clear()` empties the change list
- `to_json_summary()` produces valid JSON with correct structure
- `len()` and `is_empty()` reflect actual state
- Thread safety: record from multiple threads (SessionChanges uses interior mutability)

### TurnSnapshot (per-turn file state capture)
- `new()` creates empty snapshot
- `snapshot_file()` with a real temp file captures content
- `snapshot_file()` with non-existent file records nothing (no panic)
- `record_created()` marks a file as created-this-turn
- `is_empty()` returns true for new, false after snapshot or record_created
- `restore()` actually restores file content and deletes created files (use tempdir)
- `file_count()` counts both snapshot and created files

### TurnHistory (stack of turn snapshots)
- `new()` creates empty history
- `push()` adds a snapshot
- `len()` and `is_empty()` reflect state
- `undo_last(1)` restores the most recent turn (use tempdir)
- `undo_last(n)` undoes multiple turns in reverse order
- `undo_last(0)` is a no-op
- `undo_last(more_than_len)` undoes all available (doesn't panic)
- `clear()` empties history
- `pop()` returns the last snapshot

### format_changes
- Empty changes → appropriate message
- Single file change → formatted correctly
- Multiple changes → all listed
- Different ChangeKind values display correctly

## Constraints
- Only modify `src/session.rs`
- Use `tempfile::tempdir()` for any file-system tests (never touch the real repo)
- Aim for 25-35 new tests (roughly doubling coverage)
- Tests should be fast (no sleeps, no network)
