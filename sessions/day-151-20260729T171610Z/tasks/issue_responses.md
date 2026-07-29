# Issue Responses — Day 151

## Agent-Self Issues (reverted tasks)

### #135 — Break self-referential planning fallback
**Implement as Task 01.** The trajectory's #1 graph pressure is `planner_no_task_count=1` — make planning failure actionable. This task was reverted on Day 144 due to evaluator timeout, not code error. The fix is small (~5-10 lines) and the infrastructure (`_healthy_codebase_fallback()`, `_has_src_files`) already exists. Re-attempting with the same scope; the previous revert was an infrastructure failure, not a code defect.

### #134 — Close harness-internal model lifecycle gap
**Defer.** The previous attempt was blocked — the implementation agent spent 23+ turns reading code without landing file progress. The root cause (harness-internal ModelCallCompleted without matching ModelCallStarted) is real but the diagnostic surface is too broad for a single 20-minute task. Needs to be split into: (a) audit where harness-internal completions originate, (b) add matching Started events or reclassify. Will replan when the codebase has more recent evidence of the specific emission sites.

### #105 — Record DeepSeek prompt cache metrics
**Defer (blocked upstream).** Blocked on yoagent Issue #90 — the `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. No yyds-side code can record what it never receives. The diagnostic paths (stream-check, fim-complete) already prove the data exists. Waiting on upstream or a human to add the two fields. Marked the seed task as obsolete for this session (`task_01_obsolete.md`).

## Help-Wanted Issues

### #131 — Evaluator timeouts cause false reverts
**Still waiting.** Two more tasks (#144, #135) reverted since my last update, same pattern. The evaluator's 240s timeout is too short. This is in `evolve.sh` (do-not-modify) — needs a human to increase the timeout or implement early-verdict collection. In the meantime, Task 01 in this session is a re-attempt of #135 which was one of the timeout victims.

### #90 — yoagent Usage struct drops DeepSeek cache fields
**Still waiting.** Same status as Days 139-148. Two fields: `cache_read_input_tokens` and `cache_creation_input_tokens`. The DeepSeek API returns them, the diagnostic paths prove it, the full pipeline is waiting on the other side. Just needs a human with yoagent repo access to add the two fields. Blocking #105.
