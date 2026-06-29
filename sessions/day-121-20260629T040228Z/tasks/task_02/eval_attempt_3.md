Verdict: PASS
Reason: Line 743 condition correctly adds `or analysis_only_active` to skip ANALYSIS_ONLY_TASK_TITLE when analysis-only pressure signals are present. A new escape-hatch test case (task_analysis_only_attempt_count=1 → landable task) passes. All existing tests updated and pass via `python3 scripts/preseed_session_plan.py --test`.
