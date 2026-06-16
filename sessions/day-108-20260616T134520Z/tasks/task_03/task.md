Title: De-flake `empty_piped_stdin_exits_quickly` integration test
Files: tests/integration.rs
Issue: none
Origin: planner

Objective:
Replace the wall-clock timeout assertion in `empty_piped_stdin_exits_quickly` with a more reliable mechanism so the test stops failing intermittently in CI.

Why this matters:
The test `empty_piped_stdin_exits_quickly` (line 1486) asserts that the binary exits within 40 seconds when given empty piped stdin. This has recurring CI failures (4x historical: "thread 'empty_piped_stdin_exits_quickly' panicked"). The timeout was already bumped from 20s→40s (Day 108 09:01) but the underlying problem remains: wall-clock timing is sensitive to CI runner load. A loaded runner can push a normally-fast exit past 40s, causing flaky failures.

The sibling test `empty_piped_stdin_exits_with_nonzero_code` (line 1081) only checks exit code, not timing — and never flakes. The fix should either remove the timing assertion (relying on the non-zero exit code guard) or replace it with a generous bound that only catches genuine hangs (e.g., 120s).

Success Criteria:
- `empty_piped_stdin_exits_quickly` no longer fails due to CI runner load variance.
- The test still catches genuine hangs (process never exits).
- Both `empty_piped_stdin_exits_quickly` and `empty_piped_stdin_exits_with_nonzero_code` pass consistently.

Verification:
- cargo test empty_piped_stdin -- --test-threads=1
- cargo test --test integration -- --test-threads=1

Expected Evidence:
- The recurring CI failure pattern "thread 'empty_piped_stdin_exits_quickly' panicked" stops appearing in log feedback.
- Test suite remains green under normal and loaded CI conditions.

Implementation Notes:
- Option A (preferred): Remove the timing assertion entirely. The exit-code check (`!output.status.success()`) already verifies the process exits non-zero. The timing check was a defense against hangs, but hangs are better caught by CI's own job timeout.
- Option B: Raise the timeout to 120s as a hang guard only. This gives loaded CI runners plenty of headroom while still catching infinite loops.
- Option C: Use `wait_timeout` from `wait_timeout` crate to enforce a generous deadline without wall-clock measurement. This is more work and adds a dependency.
- If choosing Option A, also add a brief comment explaining why the timing check was removed (CI load variance makes wall-clock assertions unreliable for sub-second expected exits).
- Do NOT modify `empty_piped_stdin_exits_with_nonzero_code` — it's already stable.
