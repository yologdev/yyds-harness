# Task blocked by no-progress implementation attempts

The implementation agent used the allowed attempts without landing file progress
or emitting TASK_TERMINAL_EVIDENCE.

This task should be replanned with narrower scope, clearer owning files, or
stronger pre-confirmed evidence before another implementation attempt.

Recent transcript tail:
```
**Scenario: Normal piped mode with empty_input
1. Session starts → RunStarted (run_id=RUN-A)
2. stdin is empty → stash_diagnostic_error("empty_input") → exit_with_state(1) → mark_run_completed_with_error → RunCompleted (run_id=RUN-A, status=error)
3. Result: RunStarted and RunCompleted exist for RUN-A → they pair → RUN-A is NOT in run_incomplete_ids

**Scenario: append_terminal_state_events retroactively adds RunCompleted for orphaned runs
1. Some old run has RunStarted but no RunCompleted → it's in run_incomplete_ids
2. append_terminal_state_events adds RunCompleted with failure detail → now it pairs → no longer in run_incomplete_ids

So the filtering seems correct for the normal case. But wait — what about the case where append_terminal_state_events adds RunCompleted with some error_detail that is NOT input-validation but is still a pre-agent error? For example, a "crash" or "timeout" that happens before the agent runs?

Actually, let me re-read the task more carefully:

> When `deepseek_model_call_incomplete_count` includes input-validation exits, the harness sees "incomplete model calls → need lifecycle repair" and selects tasks to fix lifecycle recording.

Can input-validation exits contribute to `deepseek_model_call_incomplete_count`? 

`deepseek_model_call_incomplete_count` counts ModelCallStarted without ModelCallCompleted. An input-validation exit happens BEFORE any model call. The model is never called. So ModelCallStarted is never emitted. So `deepseek_model_call_incomplete_count` is never affected.

OK wait — maybe the issue is more subtle. What if `state_run_incomplete_count` is inflated by input-validation exits that have RunStarted but are missing RunCompleted due to crashes? Let's think...

Actually, I think the existing code is already correct and the task is trying to fix something that doesn't need fixing. Let me verify by looking at the actual gnome computation paths one more time.

But wait — there's one thing I haven't checked. The `state_cache_metrics` function in `log_feedback.py` reads from ONE session directory's events.jsonl, while `summarize_state_gnomes.py`'s `summarize_state_lifecycle` reads from a list of events. They both compute their own lifecycle stats differently.

In `state_cache_metrics` (log_feedback.py line 1227):
- Line 1277: `incomplete_count = len(started_runs - completed_runs) + max(unkeyed_starts - unkeyed_completions, 0)` — this counts MODEL CALL incompletes
- Line 1279: `run_incomplete_ids = run_started - run_completed` — this counts RUN incompletes

The `run_incomplete_ids` are computed as `run_started - run_completed`. If a run has RunStarted AND RunCompleted (even in error), it pairs and is not in the incomplete set. So the filtering for input-validation exits is already correct: they don't contribute to `state_run_incomplete_count`.

For `deepseek_model_call_incomplete_count`: input-validation exits don't generate ModelCallStarted, so they can't contribute.

And the lesson emission at lines 2060-2075 already uses:
- Line 2060: `state_run_incomplete_count` — which is correctly filtered by pairing
- Line 2068: `state_run_unmatched_non_validation_completed_count` — which is already filtered by `is_input_validation_completion` at lines 1281-1288
- Line 2076: `deepseek_model_call_incomplete_count` — which is not affected by input-validation

So the filtering is ALREADY in place. The task seems to be asking for something that already exists.

But wait — the task says:
> `is_input_validation_completion()` exists at summarize_state_gnomes.py:464 — it detects RunCompleted(error) with detail="empty_input" or "invalid_input:..." — but it's only used locally, not plumbed through to log_feedback.py's lifecycle lesson emission

But I've confirmed that `is_input_validation_completion` ALSO exists in log_feedback.py (line 1216) and IS used there (line 1284). So the task's evidence is outdated — the function IS plumbed through.

And:
> In log_feedback.py, find where lifecycle lessons are emitted (search for "incomplete model call" or "incomplete run" in the lesson text). Before emitting those lessons, filter out events where `is_input_validation_completion()` returns true.

Looking at the lesson emission:
- Line 2060: `state_run_incomplete_count` — this checks if RunStarted exists without RunCompleted. Input-validation exits have both, so they don't contribute. No filtering needed.
- Line 2076: `deepseek_model_call_incomplete_count` — this checks if ModelCallStarted exists without ModelCallCompleted. Input-validation exits don't emit ModelCallStarted. No filtering needed.

So the filtering is already correct. The task appears to be already satisfied.

Per the evolve skill rule:
> 6. If current code already satisfies the task, do not finish with analysis alone. Either add or strengthen a focused regression test, docs, or state evidence that makes the satisfied behavior mechanically verifiable, or write a clear obsolete-task note explaining the proof and stop without claiming a landed implementation.

So I should either:
1. Add a regression test that verifies the filtering stays correct, or
2. Write `session_plan/task_01_obsolete.md`

Given the task says I must have `git diff --name-only` non-empty or write the obsolete note, and since the existing code already satisfies this, I should write the obsolete note.

But wait — the checkpoint says:
> By your third tool turn in this retry, do exactly one of:
> - write or edit a focused regression test in the listed task surface,
> - edit one listed task-scope source file,
> - write session_plan/task_01_obsolete.md with exact evidence that the task is already satisfied,
> - write session_plan/task_01_blocked.md with the concrete blocker and missing evidence.

Let me think about whether there's a small regression test I could add. The task says `python3 -m unittest scripts.test_append_terminal_state_events scripts.test_task_lineage_feedback` as the verifier. Let me check if there's already a test file for log_feedback or summarize_state_gnomes.

Actually, let me check what test files exist:
────────────────────────────────────────────────────────────────────────────────


  ╭─ Turn 25 ────────────────────────────────────────────────────────────╮
  ▶ $ git ls-files scripts/test_*.py ✓ (455ms)


```
