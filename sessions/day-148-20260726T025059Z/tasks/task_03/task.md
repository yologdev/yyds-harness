Title: Add diagnostic detail to ModelCallCompleted when model returns zero tokens
Files: src/prompt.rs
Issue: none
Origin: planner

Evidence:
- Day 147 16:58 trace shows 2 `ModelCallCompleted` events with `tokens=in:0 out:0 cache_read:0 cache_write:0` — the model either wasn't called or returned nothing
- Day 147 had 3 empty sessions, all with `planning_failed` — the assessment phase produced no useful output
- Trajectory: `deepseek_model_call_abnormal_completed_count=2` with lifecycle causes: `model_abnormal/model_completion_without_start=2`
- The state trace confirms these are real agent model calls (not harness-internal evt-harness events) — the model returned zero tokens and the session failed silently
- Current ModelCallCompleted payload includes `input_tokens`, `output_tokens`, `status`, `thinking_observed`, `collected_text_present` — but when all are zero/false, there's no diagnostic field explaining WHY (API error? timeout? empty stream? cancelled?)

Edit Surface:
- src/prompt.rs

Verifier:
- cargo test prompt
- cargo build

Fallback:
- If ModelCallCompleted events with 0/0 tokens already carry diagnostic detail (e.g., `error_detail` is populated), mark this task obsolete.
- If the 0/0 events observed in Day 147 trace are exclusively from `scripts/append_terminal_state_events.py` (harness-internal retroactive lifecycle closing) rather than agent-code-path, narrow the fix to add a `harness_internal: true` flag in the retroactive payloads instead.

Objective:
When a ModelCallCompleted event has 0 input tokens and 0 output tokens, include diagnostic information in the payload explaining the likely cause, so future state traces can distinguish "model was never called" from "model was called but returned empty" from "API error prevented completion."

Why this matters:
The trajectory shows `deepseek_model_call_abnormal_completed_count=2` and `model_completion_without_start=2`. When the model returns empty, the entire session fails silently — the assessment phase produces no output, the planning phase can't seed tasks, and the harness reports "success" despite producing nothing. Currently, there's no way to distinguish WHY a model call produced zero tokens — was the API unreachable? Did it timeout? Did it return an error response? Adding diagnostic detail to the zero-token case makes the failure traceable and actionable.

Success Criteria:
- When `usage.input == 0 && usage.output == 0` in `model_call_terminal_payload()`, the payload includes an `error_detail` field like "model returned zero tokens — possible API error, timeout, or empty response"
- When tokens are non-zero, the payload is unchanged (no regression)
- `cargo test prompt` passes

Verification:
- cargo test prompt
- cargo build

Expected Evidence:
- Next session's ModelCallCompleted events with zero tokens include `error_detail` explaining the empty response
- `yyds state tail --limit 50 | grep ModelCallCompleted` on future empty sessions shows diagnostic detail
- The `deepseek_model_call_abnormal_completed_count` metric becomes more actionable because the "why" is recorded

Implementation Notes:
The change is in `model_call_terminal_payload()` at line 42 of `src/prompt.rs`. After the existing payload construction (around line 52, after `collected_text_present` is set), add:

```rust
if usage.input == 0 && usage.output == 0 {
    payload["error_detail"] = serde_json::json!(
        "model returned zero input and output tokens — possible API error, timeout, or empty response"
    );
}
```

This is a ~3-line addition. It doesn't change the function signature, doesn't affect non-zero cases, and uses the existing `error_detail` field pattern that's already set for explicit errors at line 53-55.

Alternatively, the check could be done at the call site (around line 953-966) before calling `model_call_terminal_payload()`, passing `Some("zero tokens — possible API error")` as the `error_detail` argument. Either approach works; the call-site approach is slightly cleaner because it doesn't add logic to the payload builder.
