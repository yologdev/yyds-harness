Title: Wire crash reporter into prompt execution errors
Files: src/prompt.rs
Issue: none
Origin: planner

Objective:
Make prompt execution failures visible in the state system by calling `stash_diagnostic_error` at key error points in `run_prompt_with_changes()` and `run_prompt_auto_retry()` so the state graph can distinguish prompt-level failures (API errors, retry exhaustion, model failures) from infrastructure failures.

Why this matters:
`src/prompt.rs` (2,743 lines) is the central prompt execution engine — it handles every user turn, every tool-call loop, every retry. It sets `api_error` in 6 places but never calls `stash_diagnostic_error`. This means prompt execution failures are a complete blind spot in the state system. The assessment explicitly lists "prompt execution errors" as the second uncovered gap. When a prompt fails with an API error or retry exhaustion, the current state graph shows nothing — the failure is silent in diagnostics.

Success Criteria:
- API errors in `run_prompt_with_changes()` emit `"prompt: api_error: {reason}"` diagnostics
- Retry exhaustion in `run_prompt_auto_retry()` emits `"prompt: retry_exhausted: {reason}"`
- Tool execution errors that cause prompt failure emit `"prompt: tool_error: {tool_name}: {error}"`
- Successful prompt paths produce no new diagnostics
- Existing prompt behavior is unchanged

Verification:
- cargo check
- cargo test prompt
- cargo test -- --test-threads=1

Expected Evidence:
- Task lineage links `src/prompt.rs` to this task
- Future state events show `prompt: api_error:`, `prompt: retry_exhausted:`, `prompt: tool_error:` diagnostic kinds
- `/state crashes` can distinguish prompt-level failures from transport/connection failures

Implementation Notes:
- Focus on the 3 highest-value failure points:
  1. `api_error = Some(...)` assignments (lines ~1190, ~1244, ~1445, ~1455)
  2. `last_tool_error` paths that cause prompt termination
  3. Retry-exhausted paths in `run_prompt_auto_retry` (line ~1271)
- Use the existing `crate::state::stash_diagnostic_error(...)` import pattern
- The `last_api_error` variable in `run_prompt_stream_json` (line ~1664) is also a candidate
- Keep each diagnostic call minimal — one `format!()` with a prefix and the error
- Do not modify prompt logic, only add diagnostic emissions at existing error sites
- Each file touch must add at most 3-4 diagnostic calls to keep the change surgical
