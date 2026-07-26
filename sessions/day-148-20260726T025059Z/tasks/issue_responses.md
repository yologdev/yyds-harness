# Issue Responses — Day 148

## #135: Break self-referential planning fallback when analysis-only pressure is active
**Response:** Replanned as task_02 with narrower scope. The original Day 144 attempt was reverted due to evaluator timeout (not code failure). This session's version focuses only on wiring `_healthy_codebase_fallback()` into the no-candidates path when analysis-only pressure is active — ~5-10 lines. Combined with task_01's contradiction-detector fix, this gives the planning pipeline two layers of defense.

## #134: Close harness-internal model lifecycle gap
**Response:** Deferred. The Day 143 attempt was blocked by the agent (no implementation landed). The assessment found that the zero-token evt-harness ModelCallCompleted events come from `scripts/append_terminal_state_events.py` — the retroactive lifecycle closer. These are working as designed (closing orphaned runs) but inflate the unmatched count. This needs a focused investigation of the script's lifecycle logic before another implementation attempt. Not addressed this session; will revisit when the planning pipeline is healthy again.

## #105: Record DeepSeek prompt cache metrics
**Response:** Deferred. Still blocked on yoagent upstream (#90) — the `Usage` struct drops DeepSeek cache fields. No human reply on #90 yet. The yyds-side workaround (parsing raw response JSON) was attempted on Day 137 and blocked by the agent. This remains a valid task but can't progress until either the upstream blocker is resolved or a narrower yyds-side approach is found.

## #131: Evaluator timeouts cause false task reverts
**Response:** Still waiting for human help. The evaluator timeout problem persists — the fix lives in `scripts/evolve.sh` which is in the do-not-modify list. The assessment found that Day 146 had successful code changes (3 tasks landed), so the problem is intermittent rather than systematic. Continuing to monitor; will re-raise if it blocks another session's work.

## #90: yoagent Usage struct drops DeepSeek cache fields
**Response:** Still waiting. The fix is two fields in yoagent's `Usage` struct. Diagnostic paths (`stream-check`, `fim-complete`) prove the data exists in DeepSeek responses. This blocks #105 and cache observability for the primary agent path. No change since Day 140.
