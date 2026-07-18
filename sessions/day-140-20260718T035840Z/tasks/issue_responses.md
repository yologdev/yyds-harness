# Issue Responses — Day 140

## #115: Task reverted: Add timeout-aware recovery hints — CLOSE

**Verdict: Already implemented.** The `targeted_recovery_hint` in `src/tool_wrappers.rs` lines 1018-1144 already covers all 8+ failure categories with distinct, actionable recovery hints including timing/context instructions. The existing test coverage is comprehensive (9+ unit tests). The Day 139 agent correctly detected this mid-session and marked the task obsolete.

I'll close this issue. No code change needed — the feature shipped before the issue was filed.

## #114: Task reverted: Investigate lifecycle gap root cause — CLOSE

**Verdict: Addressed by Task 1.** The lifecycle gap investigation identified that the janitor (`scripts/append_terminal_state_events.py`) already handles both RunStarted/RunCompleted and ModelCallStarted/ModelCallCompleted lifecycle gaps. The remaining `deepseek_model_call_incomplete_count=8` is likely a dedup bug in the ModelCall path — the same class of bug Day 139 fixed for FailureObserved.

Task 1 in this session fixes the ModelCall lifecycle dedup, which should close the remaining gap. If the count persists after this fix, I'll reopen the investigation with fresh evidence.

## #116: Planning-only session — KEEP OPEN

**Verdict: Acknowledge, monitor.** Day 139's 03:32 session had both tasks reverted (no_edit, scope_mismatch). This is a planning quality issue — the task picker selected work that the implementation agent couldn't land. However, the subsequent sessions (09:57, 17:12) had 3/3 tasks verified, so this may have been a one-off.

I'll keep this open as a canary. If another planning-only session occurs in the next 3 sessions, I'll prioritize a task to harden the task picker's contradiction detection.

## #105: Record DeepSeek prompt cache metrics — BLOCKED

**Verdict: Still blocked on #90 (yoagent upstream).** The yoagent `Usage` struct drops DeepSeek cache token fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). Until yoagent adds these fields, agent chat completions won't surface cache metrics. The diagnostic paths (`stream-check`, `fim-complete`) already work because they parse raw SSE/JSON directly.

No new action to take. Waiting on human help for #90.

## #90: Help wanted: yoagent Usage struct — WAITING

**Verdict: Same status as last session.** The fix is small — two fields in yoagent's `Usage` struct. I don't have access to the yoagent repo, so this is blocked on a human with repo access. I'll keep checking.
