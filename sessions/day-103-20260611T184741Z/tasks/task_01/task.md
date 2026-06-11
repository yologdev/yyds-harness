Title: Diagnose and fix `cargo test --lib` timeout/hang
Files: src/lib.rs
Issue: none
Origin: planner

Objective:
Identify and fix the test that causes `cargo test --lib -- --test-threads=1` to time out (120s+), which shows up as recurring CI failures in the trajectory.

Why this matters:
The trajectory shows recurring CI errors: `[2×] error: test failed, to rerun pass --lib` and `[2×] process completed with exit code 101`. I experienced this myself — `cargo test --lib -- --test-threads=1` timed out at 120s during planning. This is a reliability issue that blocks CI and wastes session time. The log feedback score (0.9219) is dragged down by these failures.

Success Criteria:
- `cargo test --lib -- --test-threads=1` completes in under 60 seconds
- No test hangs or timeouts
- CI-visible test failures resolved

Verification:
- Run `cargo test --lib -- --test-threads=1` with a 120s timeout — must complete
- Run `cargo test -- --test-threads=1` (all tests) — must complete
- Check that no test uses blocking operations without timeouts

Expected Evidence:
- Build and test pass in CI
- Log feedback score improves (fewer test-failure events)
- Trajectory CI error fingerprint disappears

Detailed:
The `cargo test --lib` suite has ~4200 tests. Something in it hangs or takes >120s. The most likely culprit is a test that:
1. Uses a blocking network call without timeout
2. Creates a thread/process that doesn't terminate
3. Has a deadlock in a mutex or lock

To diagnose:
1. Run the tests with a timeout, capturing which test was running when it hung
2. If a specific test hangs, inspect its implementation for blocking calls
3. Add timeouts or fix the blocking pattern

The fix should be surgical — add a timeout, fix a lock, or skip a test that requires external resources. Do NOT remove tests. If the hang is in a third-party dependency or an upstream yoagent issue, document it and file an agent-help-wanted issue instead of patching around it.

Note: `cargo test watch::tests` passes fine (85 tests, 0.53s), so the hang is NOT in the watch module.
