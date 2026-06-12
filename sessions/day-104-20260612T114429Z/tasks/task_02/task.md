Title: Fix `state why last-failure` windowing blind spot
Files: src/commands_state.rs
Issue: none
Origin: planner

Objective:
When `state why last-failure` (or any failure ID search) fails because the target event lies outside the default 200-event scan window, produce a diagnostic that acknowledges the windowing limitation instead of misleading users into thinking no failures exist.

Why this matters:
The assessment found that `state why last-failure --limit 200` (the default) silently hides failures beyond the window. The error message says "No failure data to diagnose" even when failures exist 201+ events back. This is a silent data-loss pattern — users get false negatives about their failure history. Since `state why` is the primary self-diagnostic entry point for the harness, false negatives undermine trust in the entire state layer.

Success Criteria:
- When a failure ID/search term isn't found AND the events were windowed (limit > 0, events.len() >= limit), the error message includes: "the target may exist outside the {limit}-event scan window — retry with --limit 0"
- When no failures exist anywhere (full scan confirms it), the existing "No failure data to diagnose" message stays.
- The existing `--limit 0` hint in the footer is preserved but no longer the only clue.
- `cargo test` passes all existing tests.

Verification:
- `cargo test --lib commands_state -- --test-threads=1`
- `cargo build`
- Manual: `./target/debug/yyds state why last-failure` in a repo where the last failure is beyond 200 events should show the new windoing warning.

Expected Evidence:
- State events from this task link to `src/commands_state.rs`.
- Future `state why` error output includes the windowing hint when relevant.
- The diagnostic gap reported in the assessment is closed.

Implementation:
In `src/commands_state.rs`, modify `handle_why` (line 836) to pass the windowing context into the error path, or modify `build_why_report` (line 2563) to accept an optional windowing parameter.

Option A (simpler — modify `handle_why`):
- After `build_why_report` returns Err, check: if `limit > 0 && events.len() >= limit`, append a windowing hint to the error before printing.
- This keeps `build_why_report` unchanged (minimizes test impact).

Option B (cleaner — modify `build_why_report`):
- Add an `Option<usize>` parameter for the scan limit.
- When target not found AND limit is Some(n) AND events.len() >= n, include the windowing hint in the error string.
- Update callers to pass the limit.

Prefer Option A — it's a 3-line addition at the Err branch in `handle_why` and touches no other callers.

The windowing hint should read:
```
{DIM}(note: only the most recent {limit} events were scanned; the target may be further back — retry with --limit 0){RESET}
```
