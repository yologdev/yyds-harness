Title: Add fallback cache metrics recording from DeepSeekUsage construction site
Files: src/deepseek.rs, src/state.rs
Issue: #61, #62
Origin: planner

Evidence:
- `yyds deepseek cache-report` returns "no DeepSeek cache metrics found" — zero CacheMetricsRecorded events across 70,000+ events.
- The recording infrastructure exists: `state::record_cache_metrics()` at src/state.rs:525 takes `&yoagent::Usage` and is called from src/prompt.rs:956 inside a `ModelCompleted` handler.
- Despite existing infrastructure, zero events land. Previous implementation attempts (Day 124 Task 2, Day 125 Task 1) both failed — the agents spent 25+ turns analyzing without landing code.
- Two construction sites in src/deepseek.rs create `DeepSeekUsage` with `cache_hit_tokens` and `cache_miss_tokens` already populated from API responses: line 1703 (DeepSeek-native path) and line 1727 (OpenAI-compat fallback).
- The gap: no recording call exists at the DeepSeekUsage construction site. The `ModelCompleted` handler in prompt.rs may never fire for DeepSeek, or `yoagent::Usage.cache_read` may always be 0.
- The simplest fix: add a new function `state::record_cache_metrics_direct(model, hit_tokens, miss_tokens)` that records directly from primitive values, bypassing `yoagent::Usage`. Call it from both DeepSeekUsage construction sites.

Edit Surface:
- src/deepseek.rs (add recording call after DeepSeekUsage construction at lines 1703 and 1727)
- src/state.rs (add `record_cache_metrics_direct` function, ~15 lines)

Verifier:
- cargo build && cargo test --lib state
- cargo test --lib deepseek
- After build: grep for "record_cache_metrics_direct" in src/state.rs and src/deepseek.rs to confirm the calls exist

Fallback:
- If the recording sites in deepseek.rs don't have access to the model name string, extract it from the function context (the model name is available in the calling functions).
- If `crate::state` isn't importable from deepseek.rs, check existing imports and add `use crate::state;` if needed.
- If both construction sites are in async functions where recording is awkward, pick just one site (the OpenAI-compat fallback at line 1727, which processes all providers).
- Do NOT analyze why the existing prompt.rs:956 call doesn't fire. Just add the direct recording path.

Objective:
Make `yyds deepseek cache-report` return real cache hit/miss data by adding a direct recording call at the point where DeepSeek API response cache tokens are first parsed.

Why this matters:
Cache optimization is a key DeepSeek harness advantage — deterministic prompt layout + cache-friendly prefixes can dramatically reduce token costs. The `cost_per_successful_task_usd` fitness gnome can't improve without cache measurement. Two previous attempts failed because agents got lost in analysis; this task is scoped to a single mechanical change: add recording at the data source.

Success Criteria:
- A new function `record_cache_metrics_direct` exists in src/state.rs that takes (model: &str, cache_hit: Option<u32>, cache_miss: Option<u32>) and records a CacheMetricsRecorded event.
- Both DeepSeekUsage construction sites in src/deepseek.rs call this function after populating the struct.
- `cargo build && cargo test --lib state` passes.
- `cargo test --lib deepseek` passes.

Verification:
- cargo build && cargo test --lib state
- cargo test --lib deepseek

Expected Evidence:
- New function `record_cache_metrics_direct` in src/state.rs.
- Recording calls in src/deepseek.rs near lines 1703 and 1727.
- After a future session with DeepSeek API calls, `yyds deepseek cache-report` shows non-zero event count.

Implementation:
1. In src/state.rs, add a `pub fn record_cache_metrics_direct(model: &str, cache_hit: Option<u32>, cache_miss: Option<u32>)` function. It should:
   - Guard on `model.starts_with("deepseek")` (same as existing `cache_metrics_payload`).
   - Skip if both cache_hit and cache_miss are None or Some(0).
   - Build a JSON payload with model, prompt_cache_hit_tokens, prompt_cache_miss_tokens, and ts.
   - Call `record(EventType::CacheMetricsRecorded, Actor::Harness, payload)`.
2. In src/deepseek.rs, at line 1703 (DeepSeek-native path), after the DeepSeekUsage struct is created, call `crate::state::record_cache_metrics_direct(model, usage.cache_hit_tokens, usage.cache_miss_tokens)`.
3. At line 1727 (OpenAI-compat path), do the same.
4. The model name variable should be available in both contexts. If not, pass it as a parameter or extract from the genome.
5. Do not modify the existing `record_cache_metrics` function or the prompt.rs call site. This is additive only.
