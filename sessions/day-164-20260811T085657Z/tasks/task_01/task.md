Title: Fix `deepseek cache-report` to read cache metrics from ModelCallCompleted events (unblocks cache observability)
Files: src/commands_deepseek.rs
Issue: #90
Origin: planner (refined from harness seed with fresh JSONL evidence)

Evidence:
- `grep '"kind":"ModelCallCompleted"' .yoyo/state/events.jsonl | grep -v 'evt-harness'` shows agent ModelCallCompleted events with real cache data: `cache_read_tokens: 441600`, `cache_read_tokens: 184192`, `cache_read_tokens: 89728` — model_call_id format `mc-18bf...` confirms these are agent chat completions, not diagnostic paths.
- `deepseek cache-report` returns "no DeepSeek cache metrics recorded" because `build_cache_report()` (line 2039) only processes events with `event_type == "CacheMetricsRecorded"` — but agent chat completions emit `ModelCallCompleted` events, NOT `CacheMetricsRecorded` events.
- `CacheMetricsRecorded` events come only from `record_cache_metrics_direct()` which is called by diagnostic paths (stream-check, fim-complete) — agent chat completions never produce them because yoagent's Usage struct drops cache fields at the API boundary, so `record_cache_metrics()` in state.rs receives zeros.
- HOWEVER, the `ModelCallCompleted` events in the raw JSONL DO carry `cache_read_tokens` and `cache_write_tokens` — these are recorded by a different code path that preserves the full payload. The cache-report just can't see them because it filters to `CacheMetricsRecorded` only.
- `read_tail_cache_events()` (line 1964) reads ALL events without filtering — it's `build_cache_report()` that narrows to `CacheMetricsRecorded` (line 2039).
- The SQLite fallback path (`read_events_from_sqlite`, line 2007) queries only `CacheMetricsRecorded` — this ALSO needs to include `ModelCallCompleted`.

Edit Surface:
- src/commands_deepseek.rs

Verifier:
- cargo test deepseek -- --test-threads=1
- cargo build && ./target/debug/yyds deepseek cache-report (should show non-zero cache metrics)

Fallback:
- If all ModelCallCompleted events in the tail window have `cache_read_tokens=0` (truly no cache data from agent completions), the report will still show zeros. This means yoagent truly drops the fields and the JSONL events with cache data were from diagnostic paths. In that case, the task still improves correctness (cache-report now sees ALL available data sources, not just CacheMetricsRecorded), but the output won't change for agent completions.
- If the fix requires changes beyond src/commands_deepseek.rs (e.g., modifying how ModelCallCompleted events are recorded), narrow scope to the cache-report aggregation only and note remaining work.

Objective:
Make `yyds deepseek cache-report` aggregate cache metrics from BOTH `CacheMetricsRecorded` AND `ModelCallCompleted` events in the state event stream. This unblocks DeepSeek cache observability for agent chat completions by reading the cache data that already exists in the JSONL but is currently invisible to the report.

Why this matters:
Cache observability is the #1 capability gap for the DeepSeek harness. The whole point of deterministic prompt layout and stable prefixes is cache efficiency — reducing token costs for every evolution session. Without cache metrics, we're flying blind: we can't measure whether prompt layout changes help or hurt, can't detect cache degradation, and can't quantify the cost savings from our prompt stability work. The JSONL proves the data exists. The cache-report just can't see it. This is the narrowest possible fix that unblocks a critical KPI.

Success Criteria:
- `cargo test` passes (all existing tests, including the cache-report tests at line 2228+).
- `yyds deepseek cache-report` shows non-zero `hit_tokens` when `ModelCallCompleted` events with `cache_read_tokens > 0` exist in the event tail.
- The SQLite fallback path also includes `ModelCallCompleted` events.
- The existing `CacheMetricsRecorded` path continues to work (no regression for diagnostic-path cache data).
- The "no metrics" error message only appears when BOTH event types have zero cache data.

Verification:
- cargo build
- cargo test --bin yyds -- --test-threads=1
- ./target/debug/yyds deepseek cache-report (verify non-zero output when cache data exists)
- ./target/debug/yyds deepseek cache-report --json (verify JSON output is well-formed)

Expected Evidence:
- `deepseek_cache_hit_ratio` gnome becomes non-zero in the next dashboard run.
- Cost-per-task estimates become more accurate (cache savings are now visible).
- The `Graph-derived next-task pressure` section stops showing "Close yyds state and model lifecycle gaps" as a top item.

Implementation Notes:
- In `build_cache_report()` (line 2034-2064): add an `else if event_type == "ModelCallCompleted"` branch after the existing `CacheMetricsRecorded` check.
- For `ModelCallCompleted` events, read `cache_read_tokens` from payload as hit tokens and `cache_write_tokens` as miss tokens (or use `payload["input_tokens"].saturating_sub(cache_read_tokens)` to approximate misses).
- Use the same field names internally: `event_hit_tokens = cache_read_tokens`, `event_miss_tokens = cache_write_tokens` (or the saturating_sub approximation).
- The `model` field is available on `ModelCallCompleted` payloads too.
- In `read_events_from_sqlite()` (line 2007): change the SQL query to `WHERE event_type IN ('CacheMetricsRecorded', 'ModelCallCompleted')` and set `event_type` from the actual event type column (or add a second query). The simplest approach: add a second query for `ModelCallCompleted` events and merge results.
- Keep the change under 60 lines. Do not refactor or extract new functions.
- Do not modify `record_cache_metrics()` or `record_cache_metrics_direct()` in `src/state.rs` — this task is about reading existing data, not changing how it's recorded.
- The error message at line 2067-2078 should be updated to reflect that we now check both event types.
- Update the unit tests at line 2228+ to cover `ModelCallCompleted` events with cache data.
