# Task 2 blocked: Missing test dependency outside Edit Surface

## Blocker

The fixture file `eval/fixtures/local-smoke/372-deepseek-cache-metric-regression.json` must reference `cargo test` commands in its `tests` array. The regression path this task targets is:

> `record_cache_metrics_direct` is called from `src/deepseek.rs` (lines 1706, 1789) but there is **no unit test anywhere in the codebase** that verifies calling this function actually writes a `CacheMetricsRecorded` event to the state events file.

Without such a test, any fixture referencing only existing tests would be a false-positive gate — it would pass even when the cache metric workaround silently breaks.

## Evidence

- `record_cache_metrics_direct` is defined at `src/state.rs:536`. It is called from `src/deepseek.rs:1706` and `src/deepseek.rs:1789`.
- A search for `record_cache_metrics_direct` in test contexts (`src/state.rs` tests, `src/deepseek.rs` tests, `src/commands_deepseek.rs` tests) returns **zero results**. No test exercises this function.
- The existing 004 fixture (`eval/fixtures/local-smoke/004-cache-metrics.json`) tests:
  - `deepseek::tests::cache_ratio_handles_missing_and_zero_totals` — serialization, not event recording
  - `commands_deepseek::tests::cache_report_reads_canonical_yoagent_state_events` — reads events, doesn't write them via the direct path
- The SQLite projection test at `src/state.rs:4740` uses a hardcoded `CacheMetricsRecorded` event but does not call `record_cache_metrics_direct`.

## Required Test

A test like this is needed in `src/state.rs` (in the `#[cfg(test)] mod tests` block):

```rust
#[test]
fn record_cache_metrics_direct_writes_event() {
    let _state_lock = state_global_test_lock();
    reset_global_recorder_for_test();
    let dir = tempfile::tempdir().unwrap();
    let events_path = dir.path().join("events.jsonl");
    let config = StateConfig {
        enabled: true,
        fail_soft: true,
        events_path: events_path.clone(),
        store_path: None,
    };
    init_global(config, json!({})).unwrap();

    record_cache_metrics_direct("deepseek-v4-pro", Some(100), Some(50));

    let raw = std::fs::read_to_string(&events_path).unwrap();
    assert!(raw.contains("CacheMetricsRecorded"), "should contain CacheMetricsRecorded: {raw}");
    assert!(raw.contains("prompt_cache_hit_tokens"), "should contain cache_hit: {raw}");
    assert!(raw.contains("100"), "should contain hit value 100: {raw}");
}
```

## Corrected Files List

For a future task to land this fixture properly:

```
Files:
  src/state.rs  (add test)
  eval/fixtures/local-smoke/372-deepseek-cache-metric-regression.json  (new)
```

## Fallback Note

The existing 004-cache-metrics.json fixture covers serialization and report-reading but does **not** cover the full pipeline regression (raw API → direct recording → event written). This task cannot be completed with fixture-only changes.
