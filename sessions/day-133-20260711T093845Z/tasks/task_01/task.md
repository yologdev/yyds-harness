Title: Add transport error classification test for 5xx/server errors in src/deepseek.rs
Files: src/deepseek.rs
Issue: #37, #94
Origin: planner (refined from harness-seed + trajectory evidence)

Evidence:
- The assessment phase timed out after 600s (exit code 124). The harness seeded a
  diagnostic task about `scripts/preseed_session_plan.py`, but the trajectory
  shows non-src tasks get reverted by strict verification (issue #94: "task
  produced no git-visible file changes").
- Trajectory: `task_success_rate=0.0` across two Day 133 sessions — 0/2 and 1/3
  strict verified. Both sessions had tasks reverted without touching src/ files.
- Graph pressure: "Raise verified task success rate" and "Force reverted tasks
  to leave concrete evidence." This task touches a src/ Rust file and produces
  a test that passes `cargo build && cargo test` — the hardest verification gate.
- The classification functions already exist at src/deepseek.rs:1035
  (`classify_transport_error`), 1078 (`is_transient_transport_error`), and 954
  (`classify_deepseek_transport_failure`). No new code beyond the test is needed.
- Fixture `eval/fixtures/local-smoke/037-deepseek-transport-error-recovery.json`
  defines 7 held-out test names. Zero exist yet. This task adds 1 of 7.
- Issue #94 is the full reverted task (7 tests). This is the micro-reduction:
  1 test function instead of 7.

Edit Surface:
- src/deepseek.rs: Add 1 `#[test]` function at the end of the existing
  `#[cfg(test)] mod tests` block (which closes at line 4045). The test
  exercises `classify_transport_error`, `is_transient_transport_error`, and
  `classify_deepseek_transport_failure` with 5xx HTTP status codes.

Verifier:
- cargo test deepseek::tests::transport_failure_classifies_5xx_as_server_error_and_retryable -- --nocapture

Fallback:
- If `classify_transport_error` or `DeepSeekTransportErrorClass::ServerError`
  has been refactored since this plan was written, adapt test assertions to
  match the current API. If the functions tested no longer exist or have moved
  to a different module, write findings to the task lineage and mark the task
  obsolete.

Objective:
Land one verifiable transport error test in src/deepseek.rs that proves 5xx HTTP
statuses (500, 502, 503, 504) are correctly classified as ServerError and marked
as transient/retryable. This breaks the no-edit revert streak and adds the first
held-out eval coverage for transport error recovery.

Why this matters:
The trajectory shows `task_success_rate=0.0` and multiple consecutive reverts
for tasks that produce no git-visible src/ changes. This task is deliberately
minimal — one test function, one file — so it can pass strict verification and
land. Each landed test also builds toward closing issue #37 (held-out eval
coverage) and #94 (the reverted 7-test task).

Success Criteria:
- `transport_failure_classifies_5xx_as_server_error_and_retryable` test function
  exists in `src/deepseek.rs` `#[cfg(test)] mod tests` block.
- `cargo test deepseek::tests::transport_failure_classifies_5xx_as_server_error_and_retryable` passes.
- `cargo build` compiles clean.
- Existing tests continue to pass (spot-check with `cargo test --lib`).

Verification:
- cargo test --lib -- deepseek::tests::transport -- --nocapture
- cargo build

Expected Evidence:
- git diff shows new test function in src/deepseek.rs.
- Task lineage shows `cargo test` output with the new test passing.
- Fixture 037 gap count drops from 7 missing to 6 missing.

Implementation Notes:

Add the following test function at the end of the `#[cfg(test)] mod tests` block
in `src/deepseek.rs` (before the closing `}` at line 4045):

```rust
    #[test]
    fn transport_failure_classifies_5xx_as_server_error_and_retryable() {
        // 500, 502, 503, 504 all map to ServerError and are transient/retryable
        for status in &[500u16, 502, 503, 504] {
            let class = classify_transport_error(Some(*status), "");
            assert_eq!(
                class,
                DeepSeekTransportErrorClass::ServerError,
                "status {} should classify as ServerError",
                status
            );
            assert!(
                is_transient_transport_error(class),
                "status {} should be transient",
                status
            );
        }
        // Also test generic 5xx >= 500: 520 maps to ServerError
        let class = classify_transport_error(Some(520), "");
        assert_eq!(class, DeepSeekTransportErrorClass::ServerError);
        assert!(is_transient_transport_error(class));

        // Verify full classification through classify_deepseek_transport_failure
        let policy = default_transport_policy();
        let decision = classify_deepseek_transport_failure(Some(503), "service unavailable", 0, &policy);
        assert!(decision.retryable, "503 should be retryable");
        assert_eq!(decision.class, DeepSeekTransportErrorClass::ServerError);
    }
```

The `default_transport_policy()` helper and `use super::*` are already in the
test module. No new imports needed.
