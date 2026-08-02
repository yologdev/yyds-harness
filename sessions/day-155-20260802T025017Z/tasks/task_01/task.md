Title: Add test coverage for record_cache_metrics_direct zero-vs-none edge case
Files: src/state.rs
Issue: none (related: #90, #105)
Origin: planner (refined from harness-seed)

Evidence:
- Trajectory graph pressure: task_success_rate=0.5, task_verification_rate=0.5 — session needs concrete src/ change that passes cargo test.
- Assessment agent timed out (exit 124, 600s). No assessment.md produced — planning falls back to trajectory-driven task selection.
- `record_cache_metrics_direct` (src/state.rs:664) has 4 existing tests but none cover asymmetric zero values: `cache_hit=Some(0), cache_miss=Some(N)` or vice versa. The skip-at-line-668 only triggers when BOTH are zero. The edge case where one is zero and the other has data is untested.
- The model-name gating at line 665 uses `model.starts_with("deepseek")` (lowercase). The "skips_non_deepseek" test only uses "claude-4". No test verifies that "deepseek-chat", "deepseek-reasoner", or other deepseek-prefixed model names pass the gate.

Edit Surface:
- src/state.rs

Verifier:
- cargo test state

Fallback:
- If `record_cache_metrics_direct` has been removed, renamed, or the test module structure has changed since this plan was written, find the closest surviving cache-metrics test suite and add an edge-case test there. If no cache-metrics recording path exists anymore, write an obsolete note.

Objective:
Strengthen test coverage for the cache-metrics recording path. Add 1-2 focused unit tests that exercise edge cases not covered by existing tests: asymmetric zero values (one cache metric is zero, the other has data) and additional DeepSeek model name variants.

Why this matters:
Cache metrics observability is blocked on upstream yoagent changes (#90), but the direct-recording fallback (`record_cache_metrics_direct`) is the diagnostic path that DOES work. Making its test coverage stronger ensures the fallback doesn't silently break while we wait for the upstream fix. The "zero-vs-none" edge case is especially important because the DeepSeek API can return zero cache hits with nonzero cache misses (cache miss on first request, cache hit on retry).

Success Criteria:
- One new test verifying that `record_cache_metrics_direct("deepseek-v4-pro", Some(0), Some(100))` records an event (asymmetric zero — one metric is zero but the other has data, which should NOT be skipped).
- One new test verifying that `record_cache_metrics_direct("deepseek-chat", Some(100), Some(50))` records an event (alternate model name).
- All existing tests continue to pass.
- `cargo test state` passes.

Verification:
- cargo test state

Expected Evidence:
- Task lineage shows a landed src/state.rs change.
- Test suite grows by 1-2 test functions.
- No change to production code — this is test-only, so no behavior change risk.

Implementation Notes:
- Add tests in the existing `#[cfg(test)] mod tests` block near the other `record_cache_metrics_direct_*` tests (around src/state.rs line 8290+).
- Use the same test pattern as existing tests: `state_global_test_lock()`, `reset_global_recorder_for_test()`, temp dir, `init_global`, call function, assert on events file contents.
- The asymmetric-zero test: call with `Some(0), Some(100)` and assert that CacheMetricsRecorded IS present (only the `Some(0) && Some(0)` case should skip).
- The model-name test: call with `"deepseek-chat"` (or `"deepseek-reasoner"`) and assert CacheMetricsRecorded IS present.
- Do NOT modify production code. Test-only change.
- Keep each test under 40 lines following the existing pattern.
