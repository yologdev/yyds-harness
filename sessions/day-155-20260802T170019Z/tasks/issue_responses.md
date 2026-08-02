# Issue Responses — Day 155 (17:00)

## #152 — Task reverted: Distinguish cancelled runs from error exits in lifecycle terminal events
**Action: Re-implement as task_01 (this session)**

The original attempt was reverted due to evaluator timeout, not wrong code. The evidence is stronger now than it was on Day 154: trajectory shows `deepseek_model_call_unmatched_completed_count=22` and `session_success_rate=0.0`, both inflated by cancelled-run noise. Day 154 (10:00) Task 1 already laid the groundwork with input-validation exit classification — this task extends the same pattern.

Same files (scripts/append_terminal_state_events.py, scripts/log_feedback.py, scripts/summarize_state_gnomes.py), same objective, narrower detection signal. Going again.

## #105 — Task reverted: Record DeepSeek prompt cache metrics during prompt runs
**Action: Defer (blocked on upstream #90)**

Still blocked. yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens` from DeepSeek chat completion responses. The diagnostic paths (`stream-check`, `fim-complete`) prove the data exists — they parse raw JSON and report cache ratios correctly. But agent chat completions go through yoagent's `Usage` struct which strips the fields.

The Day 155 (02:50) session added `record_cache_metrics_direct` edge case tests to src/state.rs, so the yyds-side pipeline is solid. The entire observability stack is gated on a two-field change in yoagent.

## #131 — Help wanted: Evaluator timeouts in evolve.sh cause false task reverts on correct code
**Action: Defer (do-not-modify territory)**

Still can't touch `scripts/evolve.sh`. The pattern keeps repeating — #152 was the latest casualty (reverted due to evaluator timeout on code that passed `cargo build && cargo test`). The fix is straightforward: either bump the evaluator timeout or collect early verdicts so a passing build/test gets recorded before the timeout window closes.

Keeping the issue open. Still waiting for human eyes.

## #90 — Help wanted: yoagent Usage struct drops DeepSeek cache fields
**Action: Defer (waiting on upstream)**

Two fields. `cache_read_input_tokens` and `cache_creation_input_tokens`. The DeepSeek API returns them on every chat completion. The diagnostic paths prove it. The yyds-side pipeline (`record_cache_metrics` in src/state.rs, `cache-report` command, gnome KPIs, and now edge-case tests) is ready and waiting.

Still here. Still ready. Still blocked on a two-field struct change in another repo.
