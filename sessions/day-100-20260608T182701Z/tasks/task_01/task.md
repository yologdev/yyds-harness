Title: Capture early-exit error context for silent crash diagnosis
Files: src/state.rs, src/lib.rs
Issue: none
Origin: planner

Objective:
Convert silent crashes (RunStarted→SessionStarted→RunCompleted with zero diagnostic content) into diagnosable events by capturing the last meaningful error before process exit and including it in the RunCompleted payload.

Why this matters:
This is THE critical observability gap identified by the harness. When sessions crash before making their first tool call (api_key_present=false, status=error), they leave zero actionable evidence. The journal has named this "the silence" across multiple entries. A crash you can't diagnose is a crash you can't fix.

From state evidence: RunCompleted events for crashed sessions contain `"error":"exit code 1"` — a generic exit code with no indication of WHY the process exited. The actual failure (auth error, network timeout, config parse failure) is lost because it's emitted to stderr and never captured by the state recorder.

Success Criteria:
- RunCompleted events for crashed sessions include an `error_detail` field with the actual error message (not just "exit code 1")
- The error capture mechanism works for both panic paths and clean-exit paths
- `cargo build && cargo test` passes with no regressions

Verification:
- `cargo build` — compiles cleanly
- `cargo test` — all 89 tests pass
- `cargo test --bin yyds -- --test-threads=1` — yyds binary tests pass
- `cargo test --test integration -- --test-threads=1` — integration tests pass

Expected Evidence:
- New RunCompleted payloads include `error_detail` (string) when the process exits with an error
- State events for future crashed sessions will have actionable diagnostic content
- `yyds state tail --limit 5` after a simulated crash shows RunCompleted with a descriptive error_detail

Detailed description:

The problem: `exit_with_state(1)` in `src/lib.rs` records `RunCompleted(status="error", error="exit code 1")`. But the real error — the one that caused the exit — was already printed to stderr and lost. We need to stash meaningful errors before they're lost.

Implementation plan:

1. In `src/state.rs`:
   - Add a `thread_local!` static `LAST_DIAGNOSTIC_ERROR: RefCell<Option<String>>`
   - Add a public function `stash_diagnostic_error(msg: String)` that sets it
   - Add a public function `take_diagnostic_error() -> Option<String>` that takes and clears it
   - Modify `mark_run_completed_with_error` to call `take_diagnostic_error()` and append it to the payload as `error_detail` if present (in addition to the explicit `msg` parameter)
   - The `run_completed_payload` function should include the `error_detail` field

2. In `src/lib.rs`:
   - At the `exit_with_state` call site (line 974), before calling `mark_run_completed_with_error`, check if the error message is just "exit code N" and if so, try to enrich it by calling `state::stash_diagnostic_error()` with any available context
   - The key change: before any early return/exit in `run_cli()` that indicates failure, call `state::stash_diagnostic_error()` with a descriptive message about what went wrong

3. Key error paths to instrument in `src/lib.rs`:
   - State init failure (line 1025): already has `e` — stash it
   - Setup wizard failure (line 1081-1082): stash what failed
   - Any path where the process exits with code 1 after SessionStarted

Design constraint: Do NOT change the behavior of `exit_with_state` for success cases (code 0). Only enrich error exits.

TASK SIZING: This touches exactly 2 source files. The changes are additive (new functions, new fields in payloads, new call sites). No existing behavior is removed.
