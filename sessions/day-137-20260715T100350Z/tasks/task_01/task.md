Title: Fix `state summary` timeout — replace unbounded event-scan with bounded reader
Files: src/commands_state.rs
Issue: none
Origin: planner

Evidence:
- Running `./target/debug/yyds state summary` times out at 10s (confirmed by
  Phase A1 self-test this session, Day 137 10:03).
- Root cause: `handle_state_summary` at line 1307 calls `read_events(&path)`
  which loads ALL ~158K JSON events into memory just to display the total count
  in a progress note ("summary from last {limit} events of {all_count} total").
- The actual summary computation (`build_state_summary`, line 1305) already runs
  on the bounded tail from `read_tail_events(path, limit)` with default
  limit=200 — that part is fast. The unbounded `read_events` for the count is
  the sole bottleneck.
- Same root cause as the Day 132 fix for `state why`, which was resolved by
  adding bounded-window scanning (`BOUNDED_FULL_SCAN_CAP` / `DEFAULT_WHY_LIMIT`).
- Trajectory graph pressure: "Make planning failure actionable" — one session
  had planner_no_task_count=1. Unbounded state scans contribute to planning
  failures because they block diagnostic commands and waste CI wall-clock.

Edit Surface:
- src/commands_state.rs: `handle_state_summary` function (line 1298), replace
  the `read_events(&path)` call at line 1307 with a cheap line count.
  Alternatively, use `read_events_bounded` with `BOUNDED_FULL_SCAN_CAP` (100K)
  and note the truncation.

Verifier:
- cargo build
- timeout 10 ./target/debug/yyds state summary
- cargo test commands_state -- --test-threads=1

Fallback:
- If `cargo build` fails after the edit, revert and narrow the fix.
- If the 10s timeout still triggers after the fix, check whether
  `read_tail_events` or `build_state_summary` is the bottleneck (unlikely with
  limit=200, but verify).

Objective:
Make `yyds state summary` complete in under 10 seconds regardless of event-log
size, so the state diagnostic command is always available during sessions.

Why this matters:
`state summary` is a core diagnostic command used by assessment agents to check
harness health. When it times out, the assessment can't see event-type counts,
RunStarted/RunCompleted ratios, or failure counts — forcing fallback to less
informative commands. This directly degrades the quality of Phase A1 assessments
and state-driven task selection.

Success Criteria:
- `timeout 10 ./target/debug/yyds state summary` completes and prints a summary
  for the current ~158K event log.
- The total-event count is still displayed (either exact via cheap count, or
  approximate via bounded scan with a note).
- Existing `state summary` behavior is preserved for the summary output itself
  (event-type distribution, run counts, failure counts).

Verification:
- cargo build
- timeout 10 ./target/debug/yyds state summary
- cargo test commands_state -- --test-threads=1

Expected Evidence:
- `yyds state summary` completes within a few seconds (not 10s+).
- State summary output still shows event counts and type distribution.
- No regression in `build_state_summary` test assertions.

Implementation Notes:
- The fix is at `src/commands_state.rs` line 1307. Currently:
  ```rust
  let all_count = read_events(&path).map(|e| e.len()).unwrap_or(events.len());
  ```
- Replace with either:
  A) A cheap file line-count using `std::fs::read_to_string` + `.lines().count()`
     (fast for any file size, exact count).
  B) Use `crate::state::read_events_bounded(path, BOUNDED_FULL_SCAN_CAP)` and
     note that the count may be truncated if events exceed the cap.
- Option A is preferred: it gives the exact count without loading JSON.
- The `BOUNDED_FULL_SCAN_CAP` constant is already defined at line 32.
- `read_events_bounded` is at `src/state.rs` line 3266.
- This is the same class of fix as Day 132's `state why` bounded scan.
- The progress note format is: "(summary from last {limit} events of {all_count} total, use --limit 0 for full scan)" — preserve this exactly, just compute `all_count` cheaply.
- If using line-count approach, remember that `read_lines` may differ from JSON event count if events span multiple lines. However, yyds state events are always single-line JSON (one event per line), so line count == event count. Verify this assumption in the implementation.
