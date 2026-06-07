Title: First real eval run — exercise eval harness against cache-metrics fixture
Files: src/commands_eval.rs
Issue: none

## Goal
The eval harness compiles, has 368 fixtures, but has never evaluated a real patch. Run the first operational eval to close the "eval harness unused" gap — the single most important harness infrastructure gap.

## What to do

### Step 1: Validate fixtures
Run `yyds eval fixtures validate --suite local-smoke` and confirm all fixtures pass validation. If any fail, note which ones but don't fix them in this task (that's separate work).

### Step 2: Run the cache-metrics fixture
Run `yyds eval fixtures run --suite local-smoke --task cache-metrics`. This fixture (004-cache-metrics.json) tests that cache hit/miss token metrics are parseable and reportable. It has two tests:
- `cargo test deepseek::tests::cache_ratio_handles_missing_and_zero_totals -- --nocapture`
- `cargo test commands_deepseek::tests::cache_report_reads_canonical_yoagent_state_events -- --nocapture`

Expected: the eval runner executes these tests, records results in the state database.

### Step 3: Verify state recording
Run `yyds state tail` after the eval and confirm eval-related events appear (EvalStarted, EvalCompleted, etc.). If no events appear, the eval runner isn't recording state — that's a bug to fix.

### Step 4: Fix any runner bugs
If the eval runner crashes, panics, or fails to record state, fix the minimal bug in `src/commands_eval.rs`. Common failure modes to check:
- Does `run_eval()` actually call the state recorder?
- Are eval results properly serialized into state events?
- Does the runner handle test failures gracefully (don't panic on failed test — just record the failure)?

### Step 5: Verify the fixture run is repeatable
Run the same fixture twice. The second run should also succeed and record new state events (not overwrite old ones).

## Verification
- `cargo build && cargo test` must pass
- `yyds eval fixtures validate --suite local-smoke` should pass
- `yyds eval fixtures run --suite local-smoke --task cache-metrics` should complete without panic
- `yyds state tail` should show eval events

## Notes
- This is a focused task. Only fix bugs that prevent eval from running at all. Don't add features or refactor the eval framework.
- If the eval runner works perfectly on the first try, just confirm state recording and move on. The task is complete once one fixture has been run through the full pipeline and results appear in state.
- Do NOT read or modify fixture files. Only modify `src/commands_eval.rs` if the runner has bugs.
