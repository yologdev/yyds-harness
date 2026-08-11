Title: Fix `deepseek cache-report` to read cache metrics from ModelCallCompleted events
Files: src/commands_deepseek.rs
Issue: #174
Origin: planner (replaces harness-seed task_01 — cache-report fix has stronger evidence and higher impact)

Evidence:
- `grep '"kind":"ModelCallCompleted"' .yoyo/state/events.jsonl | grep -v 'evt-harness'` shows agent ModelCallCompleted events with real cache data: `cache_read_tokens: 441600`, `cache_read_tokens: 184192`. These are agent chat completions, not diagnostic paths.
- `yyds deepseek cache-report` returns "no DeepSeek cache metrics recorded" because `build_cache_report()` (line ~2039) only processes `event_type == "CacheMetricsRecorded"` — agent completions emit `ModelCallCompleted`, not `CacheMetricsRecorded`.
- `CacheMetricsRecorded` events only come from `record_cache_metrics_direct()` in diagnostic paths (stream-check, fim-complete). Agent completions never produce them because yoagent's Usage struct drops cache fields (#90).
- The cache data EXISTS in JSONL ModelCallCompleted events. The report just can't see it because it filters to `CacheMetricsRecorded` only.
- This is the #1 DeepSeek capability gap — we're flying blind on prompt cache efficiency, the primary cost optimization lever.

Edit Surface:
- src/commands_deepseek.rs

Verifier:
- cargo build && cargo test deepseek -- --test-threads=1 && ./target/debug/yyds deepseek cache-report

Fallback:
- If all ModelCallCompleted events in the tail window have cache_read_tokens=0 and cache_write_tokens=0, the report will still show zeros for agent completions but the code is still correct (it now sees all available data sources). The "no metrics" error should only appear when BOTH event types have zero cache data.
- If the fix requires changes beyond src/commands_deepseek.rs (e.g., modifying how ModelCallCompleted events are recorded), narrow scope to the cache-report aggregation only and note remaining work.

Objective:
Make `yyds deepseek cache-report` aggregate cache metrics from BOTH `CacheMetricsRecorded` AND `ModelCallCompleted` events. This unblocks DeepSeek cache observability for agent chat completions by reading cache data that already exists in the JSONL but is invisible to the report.

Why this matters:
Cache observability is the #1 capability gap for the DeepSeek harness. The whole point of deterministic prompt layout and stable prefixes is cache efficiency — reducing token costs for every evolution session. Without cache metrics, we can't measure whether prompt layout changes help or hurt, can't detect cache degradation, and can't quantify cost savings. The JSONL proves the data exists. The cache-report just can't see it.

Success Criteria:
- `cargo test` passes (all existing tests, including cache-report tests).
- `yyds deepseek cache-report` shows non-zero `hit_tokens` when ModelCallCompleted events with `cache_read_tokens > 0` exist in the event tail.
- The SQLite fallback path also includes ModelCallCompleted events.
- The existing CacheMetricsRecorded path continues to work (no regression).
- The "no metrics" error message only appears when BOTH event types have zero cache data.
- At least one new unit test covers ModelCallCompleted events with cache data.

Verification:
- cargo build
- cargo test deepseek -- --test-threads=1
- ./target/debug/yyds deepseek cache-report (verify non-zero output)
- ./target/debug/yyds deepseek cache-report --json (verify well-formed JSON)

Expected Evidence:
- `deepseek_cache_hit_ratio` gnome becomes non-zero in the next dashboard run.
- Cost-per-task estimates become more accurate (cache savings now visible).
- Task lineage shows src/commands_deepseek.rs change that passed cargo build + cargo test.

Implementation Notes:
- In `build_cache_report()`: add an `else if event_type == "ModelCallCompleted"` branch after the existing `CacheMetricsRecorded` check.
- For ModelCallCompleted events, read `cache_read_tokens` from payload as hit tokens. For miss tokens, compute `input_tokens.saturating_sub(cache_read_tokens)` — the tokens that were NOT served from cache.
- The `model` field is available on ModelCallCompleted payloads (use `payload.get("model")`).
- In `read_events_from_sqlite()`: change the SQL to `WHERE event_type IN ('CacheMetricsRecorded', 'ModelCallCompleted')` or add a second query and merge results.
- Keep the change under 80 lines. Do not refactor or extract new functions unless strictly necessary for correctness.
- Do NOT modify `record_cache_metrics()` or `record_cache_metrics_direct()` in `src/state.rs` — this task reads existing data, not changes how it's recorded.
- Update the "no metrics" error message to reflect that both event types were checked.
- Add a unit test that inserts a ModelCallCompleted event with cache_read_tokens > 0 and verifies it appears in the cache report.
