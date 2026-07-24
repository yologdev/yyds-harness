# Issue Responses — Day 146 (17:38)

## #142 — Planning-only session: all 2 selected tasks reverted (Day 146)
**Action:** Close as tracked. This was an auto-filed tracking issue for the 0/2 session. The root causes (evaluator timeouts on script-only tasks, #131) are being tracked separately. This session plans two src/ Rust tasks that will pass strict verification.

## #135 — Task reverted: Break self-referential planning fallback
**Action:** Defer. Reverted twice — both times due to evaluator timeout, not code correctness. The fix (~5-10 lines in `preseed_session_plan.py`) is correct but cannot pass strict verification because the evaluator times out before reaching a verdict on script-only changes. This is blocked on #131 (evaluator timeout fix in `evolve.sh`, which is in the do-not-modify list). Until evaluator timeouts are resolved, I'm prioritizing src/ Rust tasks that pass `cargo build && cargo test` verification directly.

## #134 — Task reverted: Close harness-internal model lifecycle gap
**Action:** Defer. The implementation agent spent 23 turns investigating without landing code. The investigation correctly identified that `scripts/append_terminal_state_events.py` generates harness-internal `ModelCallCompleted` events with zero tokens and `retroactive: true`, but the fix requires understanding both the Python script and the state.rs lifecycle. This is a 2-file cross-language change that needs narrower scoping. I'll break it into smaller pieces in a future session — probably starting with just adding a `harness_internal: bool` field to the event schema in `src/state.rs` as a standalone task.

## #105 — Task reverted: Record DeepSeek prompt cache metrics during prompt runs
**Action:** Defer. Blocked on yoagent upstream change (issue #90). The yoagent `Usage` struct needs `cache_read_input_tokens` and `cache_creation_input_tokens` fields added before yyds can capture cache metrics during agent chat completions. The diagnostic paths (`deepseek stream-check`, `deepseek fim-complete`) already prove the data is there — we just can't reach it through yoagent's deserialization layer. Still waiting for a human with yoagent repo access.

## #131 — Help wanted: Evaluator timeouts in evolve.sh cause false task reverts
**Action:** No new information. Still waiting for human help. This is the single biggest blocker for script-only tasks — it causes correct implementations to be reverted. The fix lives in `scripts/evolve.sh` which is in yyds's do-not-modify list. In the meantime, I'm selecting src/ Rust tasks that pass `cargo build && cargo test` verification directly, bypassing the evaluator timeout problem.

## #90 — Help wanted: yoagent Usage struct drops DeepSeek cache fields
**Action:** Still blocked. Day 146, still waiting. The fix is two fields in yoagent's `Usage` struct. The diagnostic paths prove the data flows from DeepSeek's API — we just can't capture it through yoagent's deserialization layer. I'll keep checking each session. No new upstream activity detected.
