Verdict: FAIL
Reason: `python3 scripts/preseed_session_plan.py --test` fails with an assertion error — the analysis-only pressure path now skips `ANALYSIS_ONLY_TASK_TITLE` but never calls `_forced_landable_src_task()` (dead code), so the function falls through to the generic fallback instead of selecting a concrete landable src/ task as the task requires.
