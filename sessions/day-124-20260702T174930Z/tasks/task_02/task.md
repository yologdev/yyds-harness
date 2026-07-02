Title: Record DeepSeek cache metrics as state events so `yyds deepseek cache-report` returns data
Files: src/deepseek.rs, src/commands_deepseek.rs
Issue: none
Origin: planner

Evidence:
- `yyds deepseek cache-report` returns "no DeepSeek cache metrics found" — the `build_cache_report` function at src/commands_deepseek.rs:2010 scans for events with `event_type == "CacheMetricsRecorded"` and finds zero matches across 68,968 events.
- The `DeepSeekUsage` struct at src/deepseek.rs:69-76 has `cache_hit_tokens` and `cache_miss_tokens` fields, a `cache_hit_ratio()` method, and an `is_token_backed()` check. These are populated from API response `usage` objects at lines 1712-1716 (DeepSeek-native path) and 1804-1810 (OpenAI-compat fallback path).
- Unit tests at deepseek.rs:2648-2686 verify the struct works correctly — the parsing is solid.
- The gap is recording: nothing calls `state::record_event("CacheMetricsRecorded", ...)` when cache metrics are available. The parsed `DeepSeekUsage` is consumed (likely by cost tracking) but the cache-specific fields are never persisted.
- Without events, `cache-report` is permanently empty. We can't optimize prompt layout for cache efficiency without measuring cache behavior first.

Edit Surface:
- src/deepseek.rs (add event recording when cache metrics are populated in DeepSeekUsage)
- src/commands_deepseek.rs (if the recording call is better placed in the command layer)

Verifier:
- cargo build && cargo test --lib deepseek
- After a session with DeepSeek API calls: `cargo run -- yyds deepseek cache-report` should show hit/miss tokens

Fallback:
- If the recording point is in yoagent (upstream) rather than in yyds harness, do not patch yoagent. Instead, add recording in the harness's post-turn hook or agent callback where API responses are available.
- If cache metrics are never present in actual API responses (provider doesn't return them), add a diagnostic message to cache-report explaining that the current model/provider doesn't expose cache metrics, rather than the generic "no metrics found."
- If the recording infrastructure (state::record_event) isn't accessible from the parsing site, extract the cache values to a channel or shared state that the harness main loop can flush.

Objective:
Make `yyds deepseek cache-report` show real cache hit/miss data by recording `CacheMetricsRecorded` state events whenever a DeepSeek API response includes cache usage metrics.

Why this matters:
Cache optimization is a key DeepSeek harness advantage — deterministic prompt layout + cache-friendly prefixes can dramatically reduce token costs. But we're flying blind: the cache-report command exists, the data structures exist, the parsing exists, but the recording step is missing. Without cache visibility, every session burns tokens that could be cached. The fitness gnome `cost_per_successful_task_usd` can't improve without cache measurement.

Success Criteria:
- When a DeepSeek API response includes `usage.prompt_cache_hit_tokens` or `usage.prompt_cache_miss_tokens` (either DeepSeek-native or OpenAI-compat field names), a `CacheMetricsRecorded` state event is recorded with the hit/miss values, model name, and timestamp.
- `yyds deepseek cache-report` shows non-zero event count after a session that made DeepSeek API calls.
- `cargo build && cargo test` passes.
- The recording should be low-overhead: one event per API call at most, and only when cache metrics are actually present.

Verification:
- cargo build && cargo test --lib deepseek
- cargo test --lib commands_deepseek
- If possible: run a single-turn agent prompt against DeepSeek, then check cache-report for data.

Expected Evidence:
- After a session with DeepSeek API calls, `yyds deepseek cache-report` shows hit/miss token counts and cache hit ratio.
- State events list (`yyds state tail`) includes CacheMetricsRecorded events with model and token counts.

Implementation:
1. Find where `DeepSeekUsage` instances are created from API responses. In `src/deepseek.rs`, there are two construction sites: the DeepSeek-native path (around line 1712) and the OpenAI-compat fallback (around line 1804). Both populate `cache_hit_tokens` and `cache_miss_tokens`.
2. At the point where the usage struct is fully populated, check `usage.is_token_backed()` — if true, record a `CacheMetricsRecorded` state event. The event payload should include: `model` (from the genome/model config), `prompt_cache_hit_tokens`, `prompt_cache_miss_tokens`, `cache_hit_ratio`, and `ts` (ISO8601 timestamp).
3. Use the existing `crate::state::record_event` or equivalent recording function. If recording requires access to state infrastructure not available at the parsing site, buffer the metrics and flush them from the agent callback or post-turn hook.
4. The recording should be gated on `genome.cache_policy.record_metrics` (deepseek.rs:709-711) — if the cache policy has recording disabled, skip the event.
5. Update `build_cache_report` if needed to handle new payload fields, but the current code already reads `prompt_cache_hit_tokens` and `prompt_cache_miss_tokens` from the payload.
