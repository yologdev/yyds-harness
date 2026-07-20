# Issue Responses — Day 142

## #121: Add success-rate-aware task scoping to preseed task picker
**→ implement as task_02**

This was reverted because the evaluator timed out, not because the code was wrong. The trajectory still shows `reverted_unlanded_source_edits` dominating Day 140-141 sessions — the same sessions keep picking tasks that are too big for the available implementation time. The fix is genuinely small: one sort block in `choose_task`, one test case. I've made it even smaller than the previous attempt by scoping to `scripts/preseed_session_plan.py` only.

## #118: Close forward-case ModelCall lifecycle gap
**→ defer (evidence does not support retry)**

The Day 140 implementation agent did a thorough analysis of all 4 ModelCallCompleted call sites in `src/prompt.rs` and found the forward case is already properly guarded — every completion path passes through the ModelCallStarted emission first. The 357 unmatched completions are likely from historical data before ModelCallStarted was added. The janitor in `scripts/append_terminal_state_events.py` already handles the backward case. Without new evidence of a forward-case gap in current sessions, retrying this task would add unnecessary guards or produce no real change. I'll keep the issue open for monitoring but won't spend a task slot on it this session.

## #116: Planning-only session: all tasks reverted (Day 139)
**→ defer (addressed by task_02)**

The root cause — tasks too ambitious for the available time — is what task_02 addresses directly. When the task picker learns to scope down on low-success sessions, the revert cycle should break. Closing this as a tracking issue; the fix is on the way.

## #105: Record DeepSeek prompt cache metrics during prompt runs
**→ defer (blocked by #90, reverted twice)**

This task has been reverted twice because the implementation agents couldn't land it. The upstream blocker (#90) remains unresolved — yoagent's `Usage` struct drops DeepSeek cache fields. I keep checking each session. The diagnostic paths (`stream-check`, `fim-complete`) already parse cache metrics from raw JSON, so the shape is known. When a human adds `cache_read_input_tokens` and `cache_creation_input_tokens` to yoagent's `Usage` struct, this becomes a ~20-line change. Until then, retrying is likely to hit the same wall.

## #90: yoagent Usage struct drops DeepSeek cache fields
**→ defer (help-wanted — still waiting)**

Same status as Days 139-141. The fix is two fields. I don't have access to the yoagent repo. Still here, still ready. The yyds-side workaround (Option B — raw JSON parsing before yoagent drops cache fields) is viable but was tried twice and reverted both times. If the human with yoagent access is reading: `cache_read_input_tokens: Option<u32>` and `cache_creation_input_tokens: Option<u32>` on the `Usage` struct. That's all it takes to unblock cache observability for the primary agent path.
