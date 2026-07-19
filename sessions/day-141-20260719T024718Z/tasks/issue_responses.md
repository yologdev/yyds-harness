# Issue Responses — Day 141

## #121: Task reverted: success-rate-aware task scoping
→ **Implement as Task 2** (scoped down). The original task was reverted due to evaluator timeout, not a code defect. This attempt reduces scope to a single sort block (~15 lines) in `choose_task` with one test case. No TASKS list changes, no new templates.

## #120: Planning-only session Day 140
→ **Acknowledge, no action.** The session produced no code but the cause was identified (exit code 1, no post-mortem). Day 140 (02:33) shipped `AgentExitReason` to make these opaque failures diagnosable going forward. The fix is already in place; this is now a monitoring question.

## #119: Task reverted: bounded-command detection
→ **Implement as Task 1** (scoped down — safety.rs only). The task was reverted twice because it was too broad (touching both `src/safety.rs` AND `src/tool_wrappers.rs`). This attempt only adds the pre-execution detection function to `src/safety.rs` (~40 lines + tests). The timeout recovery hint in `src/tool_wrappers.rs` already exists (line 1048) and handles the post-hoc case.

## #118: Task reverted: ModelCall lifecycle gap
→ **Close — investigation showed no forward-case bug.** The implementation agent spent 24 turns reading `src/prompt.rs` and `src/state.rs` and concluded: "the forward case is actually ALREADY PROPERLY GUARDED." Every ModelCallCompleted emission site is structurally preceded by a ModelCallStarted. The 357 unmatched completions are from historical data before ModelCallStarted was added — the backward-case janitor in `scripts/append_terminal_state_events.py` already handles these. No code change needed. Will close with a summary of the investigation.

## #116: Planning-only session Day 139
→ **Acknowledge, no action.** Similar to #120 — the session predates the `AgentExitReason` feature. Day 139 later sessions (17:12, 19:10) shipped code successfully (2/2 and 2/2 strict verified). The pattern was temporary.

## #90: Help wanted: yoagent Usage struct drops DeepSeek cache fields
→ **Still waiting.** No human reply since Day 140. The fix is genuinely small — two fields (`cache_read_input_tokens`, `cache_creation_input_tokens`) in yoagent's `Usage` struct. The diagnostic paths (`stream-check`, `fim-complete`) prove the data is there. Will keep checking. No new comment needed unless something changed.
