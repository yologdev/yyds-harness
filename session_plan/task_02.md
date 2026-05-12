Title: Add tests for commands_bg.rs — worst test density in the codebase
Files: src/commands_bg.rs
Issue: none

## What to do

`commands_bg.rs` has the worst test density in the codebase: 5 tests in 601 lines (0.83 per 100 lines).
It contains several pure utility functions and testable struct methods that currently have no test
coverage. Add comprehensive tests to bring it in line with the rest of the codebase.

### Functions to test

**Pure utility functions (no async, no side effects):**
- `format_elapsed(Duration)` — formats durations as "Xs", "Xm Ys", "Xh Ym Zs"
- `tail_lines(s, n)` — returns the last N lines of a string
- `truncate_command(cmd, max)` — truncates long command strings with ellipsis

**Struct methods (BackgroundJobTracker):**
- `new()` — creates empty tracker
- `exists(id)` — checks if job exists  
- `is_finished(id)` — checks if job is done
- `list()` — returns snapshot of all jobs

### Test cases

For `format_elapsed`:
- Zero duration → "0s"
- Seconds only (e.g. 42s → "42s")
- Minutes and seconds (e.g. 3m 15s → "3m 15s")
- Hours, minutes, seconds (e.g. 1h 30m 0s → "1h 30m 0s")
- Edge case: exactly 60s → "1m 0s"
- Edge case: exactly 3600s → "1h 0m 0s"

For `tail_lines`:
- Empty string → empty
- Fewer lines than requested → returns all
- More lines than requested → returns last N
- Single line, request 1 → returns that line
- n=0 → returns empty or minimal

For `truncate_command`:
- Short command (under max) → unchanged
- Long command → truncated with "..."
- Command exactly at max → unchanged
- Max of 0 or very small → handles gracefully

For `BackgroundJobTracker`:
- New tracker has empty list
- `exists` returns false for unknown ID
- `is_finished` returns false for unknown ID
- `launch` returns incrementing IDs

### Constraints

All tests should be synchronous where possible. For async methods like `get_output` and `kill`,
use `#[tokio::test]` if needed but prefer testing the synchronous methods first.
Do NOT modify any existing tests — only add new ones.
