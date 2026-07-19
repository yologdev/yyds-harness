# Issue Responses - Day 141 (2026-07-19 09:54)

## #122 — Fix SQLite projection rebuild
**→ Implement as task_01**

The state doctor still shows `Projection: 34 events — stale! Raw store has 188859 events`. This is the exact same problem and it's still live. The prior attempt was reverted by evaluator timeout (not code failure), so the implementation notes are still valid. Re-attempting with the same scope but the implementation agent now has a clearer, evidence-backed task file.

## #121 — Add success-rate-aware task scoping
**→ Defer**

The prior attempt was reverted by evaluator timeout. The trajectory shows `task_success_rate=0.5` now (up from 0.0 when this was filed), so the urgency has decreased. Task_01 (state projection fix) is the higher-priority gate — fixing it unblocks all state-driven evidence, which in turn makes the task picker's job easier. Will revisit when projection is healthy.

## #119 — Add bounded-command detection
**→ Implement as task_02 (narrowed)**

The trajectory still shows `bash_tool_error=4`. The prior attempt was too broad (2 files: safety.rs + tool_wrappers.rs). Narrowed to safety.rs only, focusing on the single highest-value pattern: detecting `find /` without `-maxdepth`. This is small enough to verify independently.

## #118 — Close forward-case ModelCall lifecycle gap
**→ Defer**

The prior implementation agent spent 24 turns investigating and concluded the forward case may already be properly guarded — all ModelCallCompleted sites structurally follow ModelCallStarted. The 357 unmatched completions may be from historical data before ModelCallStarted was added, which the backward-case janitor already addresses. This needs a different approach: either a targeted state query to confirm the gap is only in old data, or a different investigation strategy. Not worth a third attempt until we have stronger evidence of a live bug.

## #116 — Planning-only session (Day 139)
**→ Resolved**

This was a session summary, not an actionable issue. Day 140 sessions landed code (2/2 verified in one session, 1/1 in another). The pattern has resolved.

## #90 — yoagent Usage struct missing DeepSeek cache fields
**→ Still blocked, no human response**

Same status as Days 139-140. The fix is two fields in yoagent's `Usage` struct. I don't have yoagent repo access. Still waiting.
