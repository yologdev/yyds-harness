Title: Add unit test for cache metric recording and verify cache-report reads them
Files: src/state.rs, src/deepseek.rs
Issue: #76
Origin: harness-seed (refined by planner)

Evidence:
- `record_cache_metrics_direct` is called from `src/deepseek.rs:1706` (parse_chat_completion_sse) and `src/deepseek.rs:1789` (parse_fim_completion_response) to work around yoagent's Usage struct dropping DeepSeek cache fields. Zero unit tests exercise this function — no `#[test]` anywhere in the codebase calls `record_cache_metrics_direct`.
- The assessment confirms: `yyds deepseek cache-report` returns "no metrics available" despite the recording path existing.
- The reverted #76 task was blocked specifically because: "there is no unit test anywhere in the codebase that verifies calling this function actually writes a CacheMetricsRecorded event to the state events file." The fixer identified the exact missing test.
- State events file exists at `.yoyo/state/events.jsonl` (94,677 events, 91MB) — the recording infrastructure is live, just untested for this path.

Edit Surface:
- src/state.rs (add #[test] for record_cache_metrics_direct)
- src/deepseek.rs (read-only: verify call sites at lines 1706, 1789 are correct)

Verifier:
- cargo test record_cache_metrics_direct -- --test-threads=1
- cargo check

Fallback:
- If the test shows `record_cache_metrics_direct` correctly writes events and `cache-report` already reads them (just no cache data in current state), mark the task done-with-findings — the recording path works, the gap is that no cache-populated DeepSeek responses have been processed yet.
- If `record_cache_metrics_direct` is guarded by `is_initialized()` and the test's `init_global` doesn't make it pass, add a `reset_global_recorder_for_test()` call before `init_global` (the existing test pattern in state.rs).

Objective:
Add one `#[test]` that exercises `record_cache_metrics_direct` end-to-end: initialize state, call the function with known values, read the events file, and assert a `CacheMetricsRecorded` event was written with the correct model name and cache token counts.

Why this matters:
Cache metrics are the primary signal for whether DeepSeek's context caching is working — when cache hits occur, token costs drop dramatically. The `record_cache_metrics_direct` workaround is the only path that preserves DeepSeek cache fields before yoagent drops them. If this workaround silently breaks (wrong event type, wrong field names, wrong file path), cache metrics drop to zero with no alert. The `cache-report` command reports "no metrics" identically for "recording is broken" and "no cache was used" — indistinguishable without a test.

Success Criteria:
- `cargo test record_cache_metrics_direct -- --test-threads=1` passes with at least one test
- The test verifies: (a) the event is written to the events file, (b) the event type is CacheMetricsRecorded, (c) the model field matches, (d) cache_hit and cache_miss fields contain the expected values
- No regression: `cargo check` and existing state tests pass

Verification:
- cargo test record_cache_metrics_direct -- --test-threads=1
- cargo test state -- --test-threads=1  (no regression in existing state tests)
- cargo check

Expected Evidence:
- A new test function in `src/state.rs` (in the `#[cfg(test)] mod tests` block) named `record_cache_metrics_direct_writes_event` or similar
- The test follows the existing pattern: acquire `state_global_test_lock()`, call `reset_global_recorder_for_test()`, create a tempdir, call `init_global`, call `record_cache_metrics_direct`, read the events file, assert contents
- Future eval fixture #76 can depend on this test as proof the recording path works

Implementation Notes:
- The test pattern exists in `src/state.rs` — search for `state_global_test_lock` and `reset_global_recorder_for_test` to see how other state tests initialize. Use `tempfile::tempdir()` for a clean events file.
- Call `record_cache_metrics_direct("deepseek-v4-pro", Some(100), Some(50))` then read the events file and assert the JSON contains `"CacheMetricsRecorded"` with `"prompt_cache_hit_tokens": 100` and `"prompt_cache_miss_tokens": 50`.
- The function signature at `src/state.rs:536` is `pub fn record_cache_metrics_direct(model: &str, cache_hit: Option<u64>, cache_miss: Option<u64>)`.
- Do NOT modify src/deepseek.rs call sites — they are read-only context. Only add the test in src/state.rs.
- If the test fails because `init_global` was already called by another test, use the `reset_global_recorder_for_test()` pattern and serialize with `state_global_test_lock()`.
