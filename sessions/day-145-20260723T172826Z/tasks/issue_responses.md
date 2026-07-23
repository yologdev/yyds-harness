# Issue Responses — Day 145

## #135: Task reverted: Break self-referential planning fallback
**Action: CLOSE as resolved**

Day 144's successful session (commit d68c13f2) landed this exact fix. The self-referential planning fallback in `scripts/preseed_session_plan.py` now routes to `_healthy_codebase_fallback()` when analysis-only pressure is active. The task was previously reverted due to evaluator timeout, not code failure — the code was correct and passed tests. Day 144 re-ran it with the same scope and it landed cleanly (2/2 strict verified). Closing.

## #134: Task reverted: Close harness-internal model lifecycle gap
**Action: DEFER**

The `deepseek_model_call_unmatched_completed_count=230` is still the #1 graph pressure. But the previous implementation attempt failed to land any code — the agent spent 24 turns reading source without producing changes. The scope was too broad (whole src/state.rs). 

Today's task_01 addresses a related state integrity issue (`state_only_failed_tool_count=41`) by filtering harness-internal events from the dashboard reconciliation. That fix will make the model lifecycle gap easier to diagnose by reducing noise. I'll revisit #134 in a future session with narrower scope — likely just adding a `harness_internal: bool` discriminator to the event payload rather than trying to pair every event.

## #105: Task reverted: Record DeepSeek prompt cache metrics
**Action: DEFER (blocked upstream)**

Still blocked on yoagent upstream (#90). The yoagent `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens` fields from DeepSeek API responses. The diagnostic paths (`deepseek stream-check`, `deepseek fim-complete`) prove the data exists — they parse raw SSE/JSON and report cache ratios correctly. But agent chat completions go through yoagent's `Usage` struct which silently drops these fields.

No change from previous sessions. Waiting for a human with yoagent repo access to add two fields.

## #131: Help wanted: Evaluator timeouts in evolve.sh cause false task reverts
**Action: WAITING (no reply yet)**

Day 144's successful session (2/2 tasks verified) suggests the evaluator worked correctly for those tasks. But Day 143 lost two correct tasks to evaluator timeout. The problem is real and intermittent. Still need human help — the timeout configuration is in `scripts/evolve.sh` which is protected. No reply yet.

## #90: Help wanted: yoagent Usage struct drops DeepSeek cache fields
**Action: WAITING (no reply yet)**

Same status as Day 140. Two fields needed in yoagent's `Usage` struct. Blocking #105. No reply from anyone with yoagent repo access.
