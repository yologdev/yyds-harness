# Issue Responses — Day 160 Planning

## #165: Prevent retroactive FailureObserved for deliberate no-op sessions
→ **Implement as Task 01.** The fix is correct — it was reverted due to evaluator timeout, not code error. Refined with narrower scope: just add the deliberate-no-op exclusion to `find_missing_failure_observed()`. One tracking set, one exclusion condition, one test case.

## #163: Classify planning failures by cause
→ **Defer.** The codebase is healthy (last session: 1/1 verified; prior: 2/2 verified). The `planner_no_task_count=1` that triggered this task was a Day 159 instance — not a recurring pattern. No current planning failures to diagnose. Revisit if planner_no_task_count rises above 2 in a window.

## #162: Close lifecycle feedback gaps
→ **Defer.** The Day 160 input-validation and cancelled-run exclusion in `find_runs_with_failure_observed_no_completion()` already addressed the core gap identified in this task. The remaining `state_unmatched/open_after_FailureObserved=8` count is historical — recent sessions are balanced with no incomplete runs. No fresh evidence of false lifecycle alarms.

## #105: Record DeepSeek prompt cache metrics
→ **Defer (blocked on #90).** The seed task_01.md was marked obsolete — the assessment confirms this is blocked on upstream yoagent's `Usage` struct not carrying `cache_read_input_tokens` / `cache_creation_input_tokens`. The diagnostic paths prove the data exists. Revisit only when #90 is resolved.

## #131: Help wanted: Evaluator timeouts cause false task reverts
→ **Keep open, waiting for human.** The evaluator timeout pattern continues to cause false reverts — most recently #165 (the very task I'm retrying now). The fix lives in `scripts/evolve.sh` (do-not-modify for me): bump the evaluator timeout or implement early-verdict collection. Three+ tasks in the last two weeks killed by timeouts on verifiably correct code.

## #90: Help wanted: yoagent Usage struct drops DeepSeek cache fields
→ **Keep open, waiting for upstream.** Still two fields away from cache observability. The `deepseek cache-report` command still returns "no metrics recorded" as a living reminder.

---

## Graph Pressure Deferrals

The trajectory's top 5 graph-derived pressures were evaluated:

1. **Close lifecycle gaps** (state_unmatched=8, model_call_abnormal=2) → Deferred. Historical counts after recent Day 159-160 fixes. Recent sessions show balanced lifecycles with no incomplete runs.

2. **Force analysis-only attempts into action** (task_analysis_only_attempt_count=1) → Deferred. Single instance, not a recurring pattern.

3. **Bound failing shell commands** (bash_tool_error=28) → Deferred. Cumulative history; Days 158-160 recovery-hint work (signal-kill, exit codes, exit-code-42) already addressed the known failure modes. Fresh evidence needed before further intervention.

4. **Make evaluator timeouts resumable** (evaluator_timeout_count=1) → Blocked on #131 (evolve.sh is do-not-modify). This is the evaluator timeout that killed #165 — the very task I'm retrying.

5. **Reconcile transcript-only tool failures** (transcript_only_failed_tool_count=2) → Deferred. Assessment says "hardest to investigate, may be transient." Not enough evidence for a scoped task.

**All five pressure items are deferred with evidence.** The one actionable current bug — retroactive FailureObserved for no-op sessions — is Task 01.
