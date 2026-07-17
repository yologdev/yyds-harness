Title: Verify and fix DeepSeek prompt cache metrics recording path
Files: src/state.rs
Issue: #105
Origin: planner

Evidence:
- `yyds deepseek cache-report` returns "no DeepSeek cache metrics recorded from agent chat completions" — the primary evolution path produces zero cache metrics events.
- The code at `src/state.rs:615-638` (`cache_metrics_payload`) has a model-name guard: `model.starts_with("deepseek")`. If the `model` string passed at `src/prompt.rs:956` doesn't start with "deepseek", the guard silently returns None and no event is recorded.
- The code at `src/prompt.rs:956` calls `record_cache_metrics(model, &state.usage)` where `model` is the `&str` parameter threaded through `handle_prompt_events`. Its value depends on the agent config and provider — it may include a provider prefix rather than a raw model name.
- The existing test `cache_metrics_payload_uses_deepseek_input_as_miss_tokens` passes with a hardcoded "deepseek-v4-pro" model name, but this doesn't exercise the actual call path from prompt.rs.
- Issue #105 was previously attempted (Day 137) and reverted — the agent spent 24 turns in static analysis without landing code. This retry is scoped to a single-file edit: add a diagnostic test + log, determine the root cause, apply a minimal fix.

Edit Surface:
- src/state.rs

Verifier:
- cargo test state::tests::record_cache_metrics_end_to_end
- cargo test state::tests::cache_metrics_payload_uses_deepseek_input_as_miss_tokens

Fallback:
- If the test reveals that `cache_metrics_payload` works correctly in isolation and the issue is purely upstream (yoagent's Usage.cache_read is always 0), record that finding as a diagnostic comment in the code and do not attempt a harness-side workaround. The task is then: add the diagnostic test, confirm the root cause, and update the issue.
- If the fix requires changes beyond src/state.rs (e.g., src/prompt.rs to fix the model string), note the dependency and stop — this task became too broad.

Objective:
Determine whether DeepSeek prompt cache metrics fail to record because of (a) a model-name guard mismatch in `cache_metrics_payload`, (b) yoagent's `Usage` struct dropping cache fields, or (c) some other cause. Apply a minimal fix if the cause is in src/state.rs; otherwise, produce a clear diagnostic finding.

Why this matters:
Cache observability for the primary evolution path is the biggest remaining blind spot in the DeepSeek harness. Without it, we cannot measure whether the stable-prefix prompt layout (a key architectural investment) actually saves costs. The task is a diagnostic step toward closing #105 — it will either fix the issue or produce the evidence needed to escalate to yoagent upstream.

Success Criteria:
- A new test `record_cache_metrics_end_to_end` exercises `record_cache_metrics` with a DeepSeek model name and realistic yoagent::Usage values, and verifies the resulting CacheMetricsRecorded event is correct.
- If the test reveals a guard mismatch (model name not starting with "deepseek"), the guard is fixed or a warning is logged to help diagnose the production path.
- If the test reveals yoagent's Usage.cache_read is the bottleneck, a clear comment is added to `cache_metrics_payload` documenting the finding.

Verification:
- cargo test state::tests::record_cache_metrics_end_to_end
- cargo test state

Expected Evidence:
- A passing end-to-end test for `record_cache_metrics` with DeepSeek model name.
- Either: CacheMetricsRecorded events start appearing after prompt runs (if a guard fix was applied), OR a diagnostic comment confirms the upstream bottleneck (if yoagent's Usage is the issue).

Implementation Notes:
- Add `record_cache_metrics_end_to_end` test in the `#[cfg(test)] mod tests` block in `src/state.rs`. Use `tempfile` (already a dev-dependency) to create a temporary events file, call `record_cache_metrics("deepseek-v4-pro", &usage)`, then read back the event and verify its payload.
- If the test passes: the code works in isolation. Add a `log::warn!` or `eprintln!` in `cache_metrics_payload` that fires when `model.starts_with("deepseek")` is false but the model string looks like it could be a DeepSeek variant (e.g., contains "deepseek" case-insensitively). This would surface the guard mismatch in production logs.
- If the test reveals the model guard is too strict (e.g., model is "provider/deepseek-v4-pro" instead of "deepseek-v4-pro"), relax the guard to `model.contains("deepseek")` after verifying with existing tests.
- Do NOT modify src/prompt.rs in this task. If the fix requires changes there, stop and report the dependency.
- This task is intentionally narrow. Do not attempt to intercept raw API responses, modify yoagent, or add cache_control headers. The goal is diagnosis, not full implementation.
