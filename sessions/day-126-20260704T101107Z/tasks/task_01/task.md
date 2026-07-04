Title: Make cache-report explain why agent chat metrics are unavailable
Files: src/commands_deepseek.rs
Issue: #64
Origin: planner

Evidence:
- `yyds deepseek cache-report` returns "no DeepSeek cache metrics found" after
  the Day 125 `record_cache_metrics_direct` fix. The fix works for FIM and
  stream-check diagnostic paths (`parse_fim_completion_response`,
  `parse_chat_completion_sse`) but neither function is called during normal
  agent chat completions. `parse_chat_completion_sse` is only invoked from
  `yyds deepseek stream-check` (src/commands_deepseek.rs:822).
  `parse_fim_completion_response` handles FIM requests, not chat.
- `build_cache_report` (line 1999) queries SQLite for `CacheMetricsRecorded`
  events — none exist for agent chat because no recording happens during that
  flow. It returns `Err("no DeepSeek cache metrics found")` at line 2038.
- The root cause is yoagent's `Usage` struct dropping DeepSeek cache fields
  (`cache_read_input_tokens`, `cache_creation_input_tokens`). Day 125 worked
  around this with direct recording at parse time, but those parse functions
  don't run in the agent chat path.
- The command currently returns a misleading "no metrics" message that implies
  the feature is broken, when the real situation is an upstream limitation with
  a working workaround for diagnostics.
- Day 126 03:47 session attempted the full wiring fix (#64) but it was reverted
  due to evaluator timeout. The fallback (honest diagnostic) is faster to
  implement and provides immediate observability value.

Edit Surface:
- src/commands_deepseek.rs: `build_cache_report` (line 1999) and
  `render_cache_report` (line 2077). Replace the opaque "no DeepSeek cache
  metrics found" error with a message that explains:
  1. Why metrics are unavailable for agent chat (yoagent upstream limitation)
  2. That `yyds deepseek stream-check` provides cache diagnostics
  3. The available diagnostic paths that DO work

Verifier:
- cargo build
- cargo test -- --test-threads=1
- cargo run -- yyds deepseek cache-report (output must explain the limitation,
  not just say "no metrics")

Fallback:
- If the message change requires more than editing the two functions
  `build_cache_report` and `render_cache_report`, stop and mark this task
  complete with whatever honest diagnostic was added.
- Do NOT attempt to wire cache metrics into the agent chat flow — that's the
  reverted #64 scope and requires yoagent upstream changes.
- Do NOT read or modify any files other than src/commands_deepseek.rs.

Objective:
Make `yyds deepseek cache-report` provide an honest, actionable diagnostic
instead of silently returning "no metrics."

Why this matters:
DeepSeek cache pricing means cache hits/misses directly affect cost. Without
cache observability for agent chat, we're blind to a major cost lever. The Day
125 infrastructure exists (`record_cache_metrics_direct`, `CacheMetricsRecorded`
events, `cache-report` command, SQLite queries) — it's all wired except for the
agent flow. An honest diagnostic tells the operator exactly what works, what
doesn't, and why — instead of a misleading empty result that looks like a bug.

Success Criteria:
- `yyds deepseek cache-report` output clearly explains why agent chat metrics
  are unavailable (yoagent `Usage` struct limitation)
- The output mentions that `yyds deepseek stream-check` provides cache
  diagnostics for the paths where recording works
- When metrics ARE present (from FIM or stream-check runs), the output still
  works correctly and shows the data
- The message "no DeepSeek cache metrics found" is replaced with specific,
  actionable information

Verification:
- cargo build && cargo test -- --test-threads=1
- Run: cargo run -- yyds deepseek cache-report
  Expected: output explains limitation, doesn't just say "no metrics"
- Run: cargo run -- yyds deepseek cache-report --json
  Expected: JSON payload includes a `limitation` or `note` field explaining why
  agent chat metrics are absent

Expected Evidence:
- `yyds deepseek cache-report` output transitions from "no DeepSeek cache
  metrics found" to an honest diagnostic
- The existing test `cache_report_aggregates_state_events` at line 2185 still
  passes (it shouldn't need changes unless the error message format changes)

Implementation:
1. In `build_cache_report` (line ~1999): when the events vector is empty after
   filtering, instead of returning `Err("no DeepSeek cache metrics found")`,
   return a result that distinguishes "no events at all" from "events exist but
   none are cache metrics." The function currently returns `Err(...)` which
   means the caller treats it as a hard error.

2. In `render_cache_report` (line ~2077): When `build_cache_report` returns the
   "no events" case, render a message like:
   ```
   DeepSeek server-side cache report
     No cache metrics recorded from agent chat completions.
     Reason: yoagent's Usage struct drops DeepSeek cache token fields
     (cache_read_input_tokens, cache_creation_input_tokens).
     Cache metrics ARE recorded for these diagnostic paths:
       - yyds deepseek stream-check  (chat completion SSE parsing)
       - yyds deepseek fim-check     (FIM completion parsing)
     Use one of those commands to populate cache metrics, then re-run this report.
   ```

3. Keep the happy path unchanged — when `CacheMetricsRecorded` events exist,
   the existing aggregation and rendering logic should work exactly as before.

4. The `--json` output path should include a `diagnostic_note` or `limitation`
   field when no metrics are available, so programmatic consumers can
   distinguish "no data yet" from "data unavailable for this path."
