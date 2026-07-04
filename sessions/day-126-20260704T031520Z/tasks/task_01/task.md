Title: Wire cache metrics recording into agent chat completion flow
Files: src/prompt.rs, src/deepseek.rs
Issue: none
Origin: planner

Evidence:
- `yyds deepseek cache-report` returns "no DeepSeek cache metrics found" after the Day 125 `record_cache_metrics_direct` fix. The fix added recording calls inside `parse_fim_completion_response` (src/deepseek.rs:1706) and `parse_chat_completion_sse` (src/deepseek.rs:1789), but neither function is called during normal agent chat completions. `parse_chat_completion_sse` is only invoked from `yyds deepseek stream-check` (CLI diagnostic, src/commands_deepseek.rs:822). `parse_fim_completion_response` handles FIM requests, not chat.
- The cache-report command (src/commands_deepseek.rs:1978) queries SQLite for `CacheMetricsRecorded` events — none exist because no recording happens during agent operation.
- `src/prompt.rs:238-249` shows `finish_prompt_epilogue` receives a `Usage` struct from yoagent, which is where cache metrics would naturally be captured — but yoagent's `Usage` struct drops DeepSeek cache fields (the reason Day 125 added `record_cache_metrics_direct` in the first place).
- Recent trajectory: 5 consecutive healthy sessions, strong task throughput — this is the right time to fix observability infrastructure.

Edit Surface:
- src/prompt.rs (wire cache metrics recording into finish_prompt_epilogue or wherever the agent has access to raw usage/cache data)
- src/deepseek.rs (may need to add/extract helpers for cache metrics extraction from whatever response shape is available in the agent flow)

Verifier:
- cargo build && cargo test -- --test-threads=1
- cargo run -- yyds deepseek cache-report (must show metrics after one agent prompt turn, or clearly report why metrics are unavailable with actionable guidance)

Fallback:
- If yoagent's `Usage` struct cannot expose cache fields and no raw HTTP interception point exists in yyds, write the best-possible diagnostic: make `cache-report` explain WHY no metrics are available ("yoagent's Usage struct drops DeepSeek cache tokens — upstream fix needed") and preserve the existing parse-path recording for FIM and stream-check diagnostics.
- If wiring would require more than 3 files, narrow to the single highest-impact file and document remaining gaps.
- Do NOT vendor, fork, or reimplement yoagent internals.

Objective:
Make `yyds deepseek cache-report` show real cache metrics from normal agent operation, or provide an honest actionable diagnosis of why it cannot.

Why this matters:
DeepSeek cache pricing means cache hits/misses directly affect cost. Without cache observability, we're blind to a major cost lever. The Day 125 infrastructure exists (`record_cache_metrics_direct`, `CacheMetricsRecorded` events, `cache-report` command, SQLite queries) — it's all wired except for the actual agent flow. Closing this gap turns dead infrastructure into live cost observability.

Success Criteria:
- After one agent prompt turn (any simple prompt), `yyds deepseek cache-report` either shows cache metrics or clearly explains why metrics are unavailable
- The message "no DeepSeek cache metrics found" is replaced with specific, actionable information
- `cargo build && cargo test` passes

Verification:
- cargo build && cargo test -- --test-threads=1
- Run a minimal agent prompt (e.g., `echo "say hello" | cargo run -- -`), then check `yyds deepseek cache-report`
- If metrics appear: verify they show cache_hit/cache_miss token counts
- If metrics don't appear: verify the output explains why and what's needed

Expected Evidence:
- `CacheMetricsRecorded` events appear in state after agent turns
- `yyds deepseek cache-report` transitions from "no metrics" to either live data or an honest diagnostic
- Task lineage shows the specific interception point chosen

Implementation:
1. Study `finish_prompt_epilogue` in `src/prompt.rs` (line 236). It receives a `&Usage` from yoagent. Check yoagent's `Usage` struct fields — does it have `cache_read_input_tokens` or similar? Check the yoagent crate source at `~/.cargo/registry/src/*/yoagent-*/src/` for the `Usage` struct definition.
2. If yoagent's `Usage` HAS cache fields: call `record_cache_metrics_direct` in `finish_prompt_epilogue` with the available cache data.
3. If yoagent's `Usage` does NOT have cache fields: check if the agent event stream (AgentEvent) carries usage info with cache fields that yyds already processes. Look for `ModelCallCompleted` or similar events in the prompt execution loop.
4. If no interception point exists: add an honest diagnostic to `handle_cache_report` / `build_cache_report` that explains cache metrics are only available from `yyds deepseek stream-check` and FIM diagnostics (which use the working parse paths), not from normal agent chat completions. Update "no DeepSeek cache metrics found" to something like "No cache metrics from agent chat completions — yoagent's Usage struct drops DeepSeek cache fields. Use `yyds deepseek stream-check` for cache diagnostics."
5. Keep the change minimal. The existing parse-path recording in `parse_fim_completion_response` and `parse_chat_completion_sse` is correct and should not be changed.
