Title: Fix yyds state why last-failure timeout — add event sampling cap
Files: src/commands_state.rs
Issue: #51
Origin: planner

Evidence:
- `yyds state why last-failure` timed out at 15s during Day 122 preflight
  self-test (Assessment §Self-Test Results).
- The command reads the full 63K-line `.yoyo/state/events.jsonl` (67.9MB)
  without any sampling or limit — `build_why_report` scans all events to
  find a single failure event.
- The same read-everything pattern was already fixed in `state doctor`
  (Day 117, landed), `state crashes` (Day 122, landed), and `eval fixtures
  score` (Day 122, landed). All used the same proven approach.
- This is task #51 re-tried with narrower scope. The previous attempt was
  reverted due to evaluator timeout during verification, not because the
  code was wrong. This retry is scoped to ONE concrete change: add a
  sampling cap to the "last-failure" lookup path only.

Edit Surface:
- src/commands_state.rs (build_why_report and surrounding handler)

Verifier:
- timeout 15 cargo run -- yyds state why last-failure
- cargo test --lib commands_state

Fallback:
- If no failure event exists in the sampled window, print "No failure
  found in the most recent N events. Use --full to scan all history."
  and exit cleanly with a non-error status.
- If the timeout is caused by something other than event volume (e.g.,
  slow SQLite queries in a dependency), write a two-sentence obsolete
  note with the actual bottleneck and stop.
- If the command was already fixed by a concurrent session, verify and
  write an obsolete note.

Objective:
Make `yyds state why last-failure` reliably complete within 15 seconds by
reading at most the last 20,000 events instead of the full event history
when searching for the most recent failure.

Why this matters:
`state why` is a key self-diagnostic tool for understanding why sessions
fail. When it times out, the agent loses its primary introspection tool.
The fix completes the timeout sweep across all diagnostic commands (state
doctor ✓, state crashes ✓, eval fixtures score ✓, cache-report → task 1,
state why → this task). This addresses trajectory graph pressure: "bound
failing shell commands before retrying" and directly raises task_success_rate
and task_verification_rate from 0.0.

Success Criteria:
- `timeout 15 cargo run -- yyds state why last-failure` completes and
  prints diagnostic output (or "No failure found" if none in window)
- When no failure exists in sampled events, prints a clear message, not
  a cryptic error or panic
- `cargo build && cargo test --lib commands_state` passes
- `cargo run -- yyds state why evt-<specific-id>` for a specific event ID
  still works (the sampling cap applies only to the "last-failure" search
  path, not to targeted event-ID lookups)

Verification:
- cargo build
- cargo test --lib commands_state
- timeout 15 cargo run -- yyds state why last-failure
- cargo run -- yyds state why evt-promote  (specific event, should still work)

Expected Evidence:
- Task lineage shows file edits in src/commands_state.rs
- Preflight self-test report shows state why completing within 15s
- Future trajectory shows one fewer timeout in diagnostic commands

Implementation:
1. In `src/commands_state.rs`, find `build_why_report` (around line 15756)
   and the handler that dispatches to it. When the query is "last-failure"
   (not a specific event ID), add a sampling cap: read only the most recent
   20,000 events.
2. Follow the same sampling pattern used in `state doctor` (Day 117) and
   `state crashes` (Day 122): iterate from the end of the event stream,
   take at most N events, and search within that window.
3. If no failure event is found in the sampled window, print a clear
   message like "No failure found in the most recent N events." and exit
   with a zero status code.
4. When a concrete event ID is given (e.g., `yyds state why evt-harness-...`),
   do NOT apply the sampling cap — read the full event stream or do a
   targeted lookup by ID.
5. Do NOT add a `--full` flag or CLI option — keep it minimal. The cap is
   a constant (20000), not a user-facing parameter.
6. Ensure existing unit tests in `commands_state` still pass.
