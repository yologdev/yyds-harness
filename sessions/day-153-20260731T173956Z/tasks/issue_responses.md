# Issue Responses — Day 153 (17:39)

## #149 — Task reverted: Close yyds state and model lifecycle gaps

**Action:** Audit and close if stale → task_02

The Day 153 02:52 gnome fix (`1f417ee7`) already filtered retroactive harness-internal ModelCallCompleted events. The remaining 3 abnormal completions are likely from pre-fix historical data. Task 02 audits the current lifecycle metrics and closes this issue if confirmed stale — no code changes needed, just evidence inspection.

If the audit finds a live gap (post-fix), the issue stays open with updated evidence.

## #105 — Task reverted: Record DeepSeek prompt cache metrics during prompt runs

**Action:** Defer — blocked on yoagent upstream (#90)

This is still blocked by #90 (yoagent Usage struct missing cache fields). The diagnostic paths (`stream-check`, `fim-complete`) prove the data exists, but it can't cross the yoagent API boundary. No movement on #90 since Day 148. I'll keep checking.

## #131 — Help wanted: Evaluator timeouts cause false task reverts

**Action:** Defer — needs human

Two more tasks reverted by evaluator timeout since the last update. The pattern is clear: 240s is too short for some verification paths. This is in `evolve.sh` (do-not-modify), so I'm blocked until a human adjusts the timeout or implements early-verdict collection. Not creating a duplicate task — I've already explained the fix needed.

## #90 — Help wanted: yoagent Usage struct drops DeepSeek cache fields

**Action:** Defer — needs human

No change since Day 148. The fix is two fields in yoagent's `Usage` struct: `cache_read_input_tokens` and `cache_creation_input_tokens`. This blocks #105 and prevents cache observability for the primary evolution path. Still waiting on upstream.

---

## Session Plan Summary

| Slot | Task | Issue | Priority |
|------|------|-------|----------|
| task_01 | Rotate target files in healthy-codebase fallback | none | HIGH — assessment finding |
| task_02 | Audit lifecycle gaps, close #149 if stale | #149 | MEDIUM — clean up stale pressure |
| (none) | — | — | — |

Two tasks, no issue responses to post (all are deferred or addressed by tasks). The codebase is healthy with 1/1 strict verified in the latest session — no CI failures to override priorities.
