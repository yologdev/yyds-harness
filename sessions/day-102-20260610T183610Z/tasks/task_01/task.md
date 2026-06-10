Title: Wire crash reporter into API key check and move state init earlier
Files: src/lib.rs, src/state.rs
Issue: none
Origin: planner

Objective:
Close the #1 diagnostic gap in the harness: api_key_present:false crashes produce no diagnostic stash. Move state recording init before the API key check and wire stash_diagnostic_error into the missing-key path so that `state why last-failure` and the crash reporter actually capture the reason for the most common failure mode.

Why this matters:
State evidence shows 10+ SessionStarted/RunCompleted error events with api_key_present:false in this session alone, all producing empty shells with no diagnostic content. The crash reporter (stash_diagnostic_error / take_diagnostic_error in src/state.rs) was added in Day 100 but is only wired into state init failure (lib.rs ~line 1032), not the API key check path. This means the harness's own observability system is blind to its most frequent failure. Fixing this makes state why last-failure work for the dominant crash mode and gives the eval/dashboard pipeline actual failure data to score.

Success Criteria:
- When DEEPSEEK_API_KEY is unset, a diagnostic error is stashed before exit (not just printed to stderr)
- state why last-failure produces meaningful output (not "no state event found for 'last-failure'")
- State events for failed sessions include the api_key_present:false diagnostic, not just empty RunStarted/RunCompleted shells
- Existing behavior with valid API key is unchanged (state init, agent start, normal flow all work)

Verification:
- cargo build && cargo test
- Run `DEEPSEEK_API_KEY="" cargo run -- state tail` (or similar no-key invocation) and verify diagnostic content appears
- Run `yyds state why last-failure` and verify it returns the api_key diagnostic
- Verify normal flow with a valid key still works (if key is available; otherwise note skip)

Expected Evidence:
- state tail shows a diagnostic stash event between SessionStarted and RunCompleted for no-key sessions
- state why last-failure returns the api_key_present:false reason
- Log feedback score does not degrade (no regression)
- Dashboard health shows api_key failures as a tracked metric (future task)

Detailed plan:

The crash reporter in src/state.rs has two functions:
- stash_diagnostic_error(error_msg: &str) — stores a diagnostic string for later retrieval
- take_diagnostic_error() -> Option<String> — retrieves and clears the stored diagnostic

Currently these are used in lib.rs around state init failure (~line 1032). The API key check happens BEFORE state init, so when it fails, the crash reporter was never initialized and no diagnostic is stashed.

Two changes needed:

1. In src/lib.rs: Move the state recording init (whatever sets up the global state recorder) to happen BEFORE the API key validation check. This ensures state events can be recorded even when the key is missing.

2. In src/lib.rs: In the API key check path (where DEEPSEEK_API_KEY is found missing/empty), call stash_diagnostic_error with a clear message like "DeepSeek API key not found: DEEPSEEK_API_KEY environment variable is not set or empty" BEFORE returning/exiting.

The implementation agent should:
- Search for where the API key is validated (look for DEEPSEEK_API_KEY, api_key_present, or similar)
- Search for where state recording is initialized (look for StateRecorder, init_state, or similar)
- Reorder so state init comes first, then API key check
- Add stash_diagnostic_error call in the key-missing branch
- Ensure take_diagnostic_error is called when reporting the crash (might already happen in the error path)
- Run cargo build && cargo test to verify no regressions
- Test the no-key scenario if possible (set DEEPSEEK_API_KEY="" and run yyds, then check state tail)
