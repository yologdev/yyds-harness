# Task 02: Add eval pipeline integration test that exercises fixture → eval flow

Title: Add eval pipeline integration test with real fixture data
Files: src/eval_fixtures.rs
Issue: none

## Problem

The eval harness has ~30 fixtures in the local-smoke suite, loaded and validated, but never exercised end-to-end against simulated agent output. The assessment calls this "the factory is built but nothing's come down the line yet." We need at least one test that:
1. Loads a real fixture from the fixture directory
2. Simulates agent output that matches what a real agent session would produce
3. Runs it through the eval pipeline
4. Asserts that the eval produces coherent, non-empty results

The existing `smoke_validate_fixture_pipeline_with_real_fixture_data` test (added in the most recent session) loads fixtures but doesn't simulate agent output. This task builds on that foundation.

## What To Do

1. **Pick a simple fixture** from the local-smoke suite. Look at `eval_fixtures.rs` for the existing `smoke_validate_fixture_pipeline_with_real_fixture_data` test — understand what fixtures are available and pick one that's self-contained (e.g., a fixture that expects a specific bash command to succeed).

2. **Write a test function** `smoke_run_fixture_through_eval_pipeline` that:
   - Loads the chosen fixture (using the existing `load_fixture_suite` or `load_fixture_suite_from`)
   - Creates simulated agent output: one or more `FixtureCommandResult` or `FixtureAgentAttemptResult` records
   - Calls `run_fixture_task` or `run_fixture_agent_attempt` (whichever is the right entry point — check the existing function signatures)
   - Asserts that the result is not an error
   - Asserts that the result contains meaningful output (not empty, has expected fields populated)

3. **If the eval pipeline needs state recording** to work properly, set up the state recorder in the test (use the pattern from existing state tests — a temp directory for state files).

4. **Keep it simple.** This is a smoke test, not a comprehensive eval suite. One fixture, one simulated outcome, one assertion that the pipeline doesn't crash and produces something.

## Verification

- `cargo build` must pass
- `cargo test` must pass, including the new test
- The test should exercise at least 2-3 functions in the eval pipeline chain (load → run → evaluate)
- If the test fails because of a real bug in the eval pipeline, fix the bug (but don't make this task about fixing the pipeline — if the bug is large, record it and skip to a simpler fixture)

## Sizing Note

This touches only `src/eval_fixtures.rs` (one file). The test should be 30-60 lines. If the eval API is hard to call from a test, wrap it in a helper function first.
