Title: Read cache metrics from ModelCallCompleted events in cache-report (unblocks DeepSeek cache observability)
Files: src/commands_deepseek.rs
Issue: #174
Origin: planner (re-attempt with narrowed scope from blocked-task evidence)

Evidence:
- `grep '"kind":"ModelCallCompleted"' .yoyo/state/events.jsonl | grep -v 'evt-harness' | head -5` shows agent ModelCallCompleted events with real cache token fields (`cache_read_tokens`, `cache_write_tokens`).
- `yyds deepseek cache-report` returns "no DeepSeek cache metrics recorded" because `build_cache_report()` (line 2039) only processes `CacheMetricsRecorded` events — ModelCallCompleted events are skipped.
- `CacheMetricsRecorded` events come only from diagnostic paths (stream-check, fim-complete). Agent chat completions emit ModelCallCompleted events with cache data in the payload, but the cache-report can't see them.
- The SQLite fallback `read_events_from_sqlite()` (line 2007) also queries only `CacheMetricsRecorded` — needs to include ModelCallCompleted too.
- Previous attempt (#174 Day 164 Task 1) was reverted because the evaluator timed out without a verdict — the code may have been correct. The blocked-task evidence has clear implementation notes.
- Trajectory shows `task_success_rate=0.5` — improving cache observability directly supports cost-per-task gnomes and prompt layout fitness measurement.

Edit Surface:
- src/commands_deepseek.rs

Verifier:
- cargo check
- cargo test --bin yyds -- --test-threads=1

Fallback:
- If all ModelCallCompleted events in the tail window have `cache_read_tokens=0` and `cache_write_tokens=0` (truly no cache data from agent completions), the report will still show zeros. This is acceptable — the fix still improves correctness (cache-report now sees ALL available data sources). The "no metrics" error message should only appear when BOTH event types have zero data.
- If the fix requires changes beyond `src/commands_deepseek.rs` (e.g., modifying how ModelCallCompleted events are recorded in `src/state.rs`), narrow scope to the cache-report aggregation only and note remaining work.
- If the existing cache-report unit tests at line 2228+ fail after the change, update them to cover the ModelCallCompleted path rather than reverting the fix.

Objective:
Make `yyds deepseek cache-report` aggregate cache metrics from BOTH `CacheMetricsRecorded` AND `ModelCallCompleted` events in the state event stream. This unblocks DeepSeek cache observability for agent chat completions.

Why this matters:
Cache observability is the #1 capability gap for the DeepSeek harness. The whole point of deterministic prompt layout and stable prefixes is cache efficiency — reducing token costs for every evolution session. Without cache metrics, we're flying blind: we can't measure whether prompt layout changes help or hurt, can't detect cache degradation, and can't quantify the cost savings from our prompt stability work. The JSONL proves the data exists. The cache-report just can't see it. This is the narrowest possible fix that unblocks a critical KPI.

Success Criteria:
- `cargo check` passes (no compilation errors).
- `cargo test --bin yyds -- --test-threads=1` passes (all existing tests, including cache-report unit tests at line 2228+).
- `build_cache_report()` processes both `CacheMetricsRecorded` and `ModelCallCompleted` events.
- `read_events_from_sqlite()` queries both event types.
- The "no metrics" error message only appears when BOTH event types yield zero cache data.

Verification:
- cargo check
- cargo test --bin yyds -- --test-threads=1
- cargo build && ./target/debug/yyds deepseek cache-report (verify non-zero output when ModelCallCompleted events with cache data exist)

Expected Evidence:
- `deepseek_cache_hit_ratio` gnome becomes non-zero in the next dashboard run.
- Cost-per-task estimates become more accurate (cache savings are now visible).
- The `Graph-derived next-task pressure` section stops listing cache observability as a gap.

Implementation Notes:
- **In `build_cache_report()`** (line 2028-2064): After the `if event_type != "CacheMetricsRecorded" { continue; }` guard, add an `else if event_type == "ModelCallCompleted"` branch. For ModelCallCompleted events, read `cache_read_tokens` as hit tokens and `cache_write_tokens` as miss tokens from the payload.
- The payload field names for ModelCallCompleted are `cache_read_tokens` and `cache_write_tokens` (not `prompt_cache_hit_tokens` / `prompt_cache_miss_tokens` which are used by CacheMetricsRecorded).
- Use the same accumulators: `hit_tokens`, `miss_tokens`, `event_count`, `by_model`.
- The `model` field is available on ModelCallCompleted payloads too — use same extraction logic.
- **In `read_events_from_sqlite()`** (line 1993-2025): Change the SQL query from `WHERE event_type = 'CacheMetricsRecorded'` to `WHERE event_type IN ('CacheMetricsRecorded', 'ModelCallCompleted')`. Set `event_type` from the row's event_type column (may need to add event_type to the SELECT). OR simpler: add a second query for ModelCallCompleted events and merge results. The simplest approach: query both types in one pass by adding event_type to the SELECT and setting it in the json! block.
- **Error message** at line 2067-2078: Update to reflect that we now check both event types. Something like: "no DeepSeek cache metrics recorded (checked CacheMetricsRecorded and ModelCallCompleted events)".
- **Unit tests** at line 2228+: Add test cases with ModelCallCompleted event shapes. A minimal test: one ModelCallCompleted event with `cache_read_tokens: 1000, cache_write_tokens: 500` and model "deepseek-chat" should produce hit_tokens=1000, miss_tokens=500, ratio≈0.667.
- Keep the change under 60 lines total. Do not refactor or extract new functions.
- Do not modify `record_cache_metrics()` or `record_cache_metrics_direct()` in `src/state.rs` — this task is about reading existing data, not changing how it's recorded.
