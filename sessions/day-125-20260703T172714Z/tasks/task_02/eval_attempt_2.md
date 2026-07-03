Verdict: PASS
Reason: `record_cache_metrics_direct` exists in src/state.rs with correct guards (deepseek-prefix check, skip if both None or both Some(0)), and both DeepSeekUsage construction sites in src/deepseek.rs (lines 1706 and 1789) call it with parsed cache tokens. Build and tests pass.
