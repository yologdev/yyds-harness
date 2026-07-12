Title: Fix state why timeout by reducing redundant event scanning passes
Files: src/commands_state.rs, src/state.rs
Issue: none
Origin: planner (from assessment bug report)

Evidence:
- Assessment Day 134: `yyds state why last-failure` timed out at 30s despite
  Day 132's fix that bounded the event window to DEFAULT_WHY_LIMIT=10,000.
  The bounded window exists but the command still exceeds reasonable runtime.
- Day 132 bounded the event scan to 10,000 lines but the processing itself
  does multiple redundant O(n) passes over the same events array:
  `build_why_report` calls `find_target_event` (1 pass), `build_state_summary`
  (4 passes), plus its own 5+ counting passes. `related_events` at line 14287
  does yet another pass. That's 10+ full scans of 10,000 JSON Values.
- The result: for a healthy session with 10,000 events, the command does
  roughly 100,000+ event inspections with JSON field access overhead. This
  plausibly accounts for the 30s timeout.
- Trajectory graph pressure #3: evaluator_unverified_count=1. When state
  diagnostics time out, the evaluator can't produce verdicts. State command
  reliability is a diagnostic gate for fitness measurement.
- Unlike the read_events_bounded path (which was fixed by Day 132's limit),
  this is a processing bottleneck in build_why_report/build_state_summary,
  not an I/O bottleneck.

Edit Surface:
- src/commands_state.rs — consolidate the redundant event scans in
  `build_why_report` and `build_state_summary` into a single pass that
  collects all needed metrics. The functions currently do separate iterations
  for run_completed_count, failure_count, error_run_completed_count,
  event_types, timestamps, and error_run_ids. Merge these into one loop.
- src/state.rs — if `read_events_bounded` has a JSON parsing performance
  issue (e.g., not using a pre-sized buffer, re-parsing strings), add a
  minor optimization. But the primary fix is in commands_state.rs.

Verifier:
- time yyds state why last-failure --limit 2000 (should complete in < 5s)
- cargo test --lib commands_state (existing tests for build_why_report)

Fallback:
- If consolidating the passes breaks an existing test or changes output format
  in a way that requires updating more than 2 test expectations, stop and
  write findings to session_plan/task_02_findings.md. Do not rewrite the
  output format — that's out of scope.
- If `cargo test --lib commands_state` times out before the fix can be
  verified, add #[ignore] to the slowest test and note it.
- If the real bottleneck is in `read_events_bounded` JSON parsing (not in the
  passes), scope the fix to state.rs only and drop the commands_state.rs
  changes.

Objective:
Make `yyds state why last-failure` complete in under 5 seconds for the
default 10,000-event window, so state diagnostics are fast enough to be
usable during evolution sessions and evaluator verdicts are not skipped due
to diagnostic timeouts.

Why this matters:
The state why command is the primary diagnostic tool for understanding
session failures. When it times out, the harness loses its ability to
self-diagnose between sessions. The evaluator also depends on state
diagnostics to produce verdicts — when they time out, evaluator_unverified
count climbs. This is a diagnostic gate that directly blocks fitness
measurement and is the only concrete Rust-source improvement opportunity
in the current trajectory window.

Success Criteria:
- `time yyds state why last-failure --limit 2000` completes in < 5 seconds.
- `time yyds state why last-failure --limit 10000` completes in < 10 seconds.
- `cargo test --lib commands_state` passes without new failures.
- No output format changes (same text, same sections, same ordering).
- The fix is purely an optimization — no behavioral changes, no new features.

Verification:
- cargo build && cargo test --bin yyds
- cargo test --lib commands_state -- --test-threads=1
- time yyds state why last-failure --limit 2000
- time yyds state why last-failure
- Compare output before/after to confirm no format changes:
  yyds state why last-failure --limit 2000 > /tmp/why_before.txt
  # after fix:
  yyds state why last-failure --limit 2000 > /tmp/why_after.txt
  diff /tmp/why_before.txt /tmp/why_after.txt  # should be identical or near-identical

Expected Evidence:
- Next trajectory shows state diagnostic commands completing without timeout.
- evaluator_unverified_count drops as verdicts are produced reliably.
- No regression in state command test suite.

Implementation Notes:
- The key optimization: merge the 4 passes in `build_state_summary` into a
  single `fold` or `for` loop that collects run_completed_count,
  run_started_count, event_types map, failure_count, min/max timestamps all
  in one iteration. Same for the counting passes in `build_why_report`.
- This is a mechanical refactor — the output format must be identical.
- Use `fold` or a mutable accumulator struct. Prefer readability over
  cleverness; the goal is correctness first, speed second.
- If `event_string` calls are the bottleneck (they do JSON field access for
  every event), consider caching the event_type string in a local variable
  during the loop.
- The `read_events_bounded` function in state.rs: check if it pre-allocates
  the Vec. If it reads 10,000 lines and pushes one at a time without
  `Vec::with_capacity`, that's a minor optimization (not the main fix).
- Do NOT change DEFAULT_WHY_LIMIT or any other constant — the fix should
  make the command fast at its current limits, not reduce the limit further.
