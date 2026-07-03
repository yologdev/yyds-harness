Title: Fix cache metrics recording — diagnose why CacheMetricsRecorded events are never emitted
Files: src/prompt.rs, src/state.rs
Issue: #61
Origin: planner

Evidence:
- `yyds deepseek cache-report` returns "no DeepSeek cache metrics found" — zero `CacheMetricsRecorded` events across 70,399 total events (Assessment §Self-Test Results).
- The recording infrastructure already exists: `state::record_cache_metrics()` at src/state.rs:525 calls `record(EventType::CacheMetricsRecorded, ...)` and is invoked from src/prompt.rs:956 inside a `ModelCompleted` event handler.
- `cache_metrics_payload()` at src/state.rs:552 maps `yoagent::Usage` fields to `DeepSeekUsage`, gated on `model.starts_with("deepseek")` and a zero-check at line 566.
- Unit tests exist at src/state.rs:3738-3755 for `cache_metrics_payload_uses_deepseek_input_as_miss_tokens` and `cache_metrics_payload_skips_non_deepseek_models`.
- Despite the function existing and being called, zero events land in the state store. The gap is either: (a) `yoagent::Usage.cache_read` is always 0 for DeepSeek (upstream mapping issue), (b) the `ModelCompleted` code path is never reached in the actual agent loop, or (c) the payload filter at line 566 silently drops all events.
- `cost_per_successful_task_usd` gnome has no cache data — cache optimization is flying blind.

Edit Surface:
- src/prompt.rs (diagnose why line 956 call produces no events; add fallback recording point if needed)
- src/state.rs (fix cache_metrics_payload mapping if the yoagent::Usage → DeepSeekUsage conversion is lossy)

Verifier:
- cargo build && cargo test --lib state
- After any API call during testing: check that CacheMetricsRecorded events appear (use `yyds state tail --limit 50 | grep -i cache` or equivalent)
- `yyds deepseek cache-report` shows non-zero event count

Fallback:
- If yoagent::Usage.cache_read is never populated for DeepSeek (upstream yoagent issue), do not patch yoagent. Instead, record cache metrics directly from the DeepSeek API response in src/deepseek.rs where `DeepSeekUsage` is constructed (line 1703 or 1727), using `state::record(EventType::CacheMetricsRecorded, ...)` directly and bypassing the yoagent::Usage conversion.
- If the ModelCompleted path is never reached, record from the next available post-turn hook or callback that has access to the usage data.
- If the model name doesn't start with "deepseek" (e.g. uses a provider alias), relax the starts_with check to also match known DeepSeek model prefixes.
- If no cache tokens are present in actual API responses (provider doesn't return them), add a diagnostic message to cache-report rather than silently showing "no metrics."

Objective:
Make `yyds deepseek cache-report` return real cache hit/miss data by ensuring CacheMetricsRecorded events are emitted whenever DeepSeek API responses include cache usage metrics.

Why this matters:
Cache optimization is a key DeepSeek harness advantage — deterministic prompt layout + cache-friendly prefixes can dramatically reduce token costs. The cache-report command, recording function, and parsing code all exist but the events never land. Without cache visibility, every session burns tokens that could be cached. The fitness gnome `cost_per_successful_task_usd` can't improve without cache measurement.

Success Criteria:
- After any session that makes DeepSeek API calls, CacheMetricsRecorded events appear in the state store.
- `yyds deepseek cache-report` shows non-zero event count and hit/miss token data.
- `cargo build && cargo test --lib state` passes.
- The fix is minimal: diagnose the specific break in the existing chain, apply one targeted fix.

Verification:
- cargo build && cargo test --lib state
- cargo test --lib deepseek
- If the fix is in the recording path (not the report path): run a single-turn agent prompt, then check `yyds deepseek cache-report`.

Expected Evidence:
- CacheMetricsRecorded events in state store after DeepSeek API calls.
- `yyds deepseek cache-report` shows model name, hit/miss tokens, and cache hit ratio.
- Task lineage shows file edits in src/prompt.rs or src/state.rs.

Implementation:
1. **Diagnose first.** Add a temporary `eprintln!` or check to determine whether:
   a. The `ModelCompleted` handler at prompt.rs:956 is ever reached (is the event firing?)
   b. `state.usage` has non-zero `cache_read` or `input` fields at that point
   c. `cache_metrics_payload` returns `Some` or `None`
   The most efficient approach: add a diagnostic that prints the usage values when the handler fires, then run a single-turn prompt. Remove the diagnostic after determining the root cause.

2. **Fix based on diagnosis:**
   - If `ModelCompleted` never fires: record cache metrics from an alternative event handler that does fire (e.g., `AgentEvent::ToolCallCompleted` after the final tool call, or a session-level hook).
   - If `yoagent::Usage.cache_read` is always 0: record directly from `DeepSeekUsage` in `src/deepseek.rs` where the struct is constructed from the raw API response (line 1703 or 1727). The raw response has the correct field names (`prompt_cache_hit_tokens`, `prompt_cache_miss_tokens`). Use `state::record(EventType::CacheMetricsRecorded, ...)` directly.
   - If `cache_metrics_payload` drops valid data: fix the filter at line 566. The current check `cache_hit_tokens == Some(0) && cache_miss_tokens == Some(0)` may be too aggressive — cache_miss_tokens maps to `usage.input` which would be non-zero for any real API call. But if `usage.cache_read` is 0 (first call, no cache) and `usage.input` is also somehow 0, the event gets dropped. Consider recording even when both are 0 (first-call baseline) or fixing the mapping.

3. **Keep it minimal.** One file change, one root cause, one fix. Do not rewrite both the recording and reporting sides. Do not add new state event types or change the cache-report UI.
