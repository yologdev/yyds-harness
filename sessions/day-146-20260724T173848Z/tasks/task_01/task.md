Title: Add unit test coverage for orphaned-run lifecycle in src/state.rs
Files: src/state.rs
Issue: none
Origin: planner (refined from harness-seed)

Evidence:
- Day 143 added `close_orphaned_run_if_needed()` with 7 unit tests — but the function's edge cases (empty state dir, corrupted events, mixed open/closed runs) may have gaps
- Day 146 (04:09) added `stash_diagnostic_error`/`take_diagnostic_error` round-trip test — diagnostic error pocket works
- Trajectory: deepseek_model_call_unmatched_completed_count shows lifecycle gaps in state recording
- The state.rs module is the core of harness event recording — better test coverage directly improves harness reliability
- Assessment: all diagnostics pass, tree is clean — this is a healthy-codebase improvement task

Edit Surface:
- src/state.rs

Verifier:
- cargo test state

Fallback:
- If `cargo test state` already has >95% line coverage or every public function already has at least one test, add a doc comment to the least-documented public function instead and verify with `cargo doc --no-deps -p yyds 2>&1 | grep -c warning`.
- If no improvement is possible (all functions well-tested and documented), write an obsolete note explaining why.

Objective:
Add one unit test or doc improvement to src/state.rs that increases confidence in state recording correctness — either a test for an edge case of a recently added function, or a doc comment clarifying a public API's contract.

Why this matters:
src/state.rs (8,387 lines) is the core of harness event recording. The trajectory shows `deepseek_model_call_unmatched_completed_count` indicating lifecycle gaps — model calls that complete without starting, or start without completing. Better test coverage of state lifecycle functions (open/close run, start/complete model call) makes these gaps harder to introduce and easier to catch. This is a small, verifiable src/ Rust change that passes `cargo build && cargo test` — breaking the analysis-only cycle.

Success Criteria:
- One new unit test or doc comment lands in src/state.rs
- `cargo test state` passes with no regressions
- The change is scoped to 5-40 lines

Verification:
- cargo test state
- cargo build

Expected Evidence:
- Task lineage shows an src/state.rs change that passes strict verification
- The change is small, focused, and verifiable
- A concrete contribution to harness reliability

Implementation Notes:
- This task replaces the generic harness-seeded "add a small improvement" with a specific focus area
- Look at functions added or modified in the last 5 days: `close_orphaned_run_if_needed()`, `stash_diagnostic_error()`, `take_diagnostic_error()`
- Check if `close_orphaned_run_if_needed()` handles: empty events[] array, a run that's already closed, multiple open runs
- Or: add a test that creates a run, records ModelCallStarted without ModelCallCompleted, then verifies `close_orphaned_run_if_needed()` closes it
- Or: improve doc comments on `StateRecorder::record()` to clarify what happens when recording during an unopened run
- Keep it simple — one test or one doc comment, not a test suite
