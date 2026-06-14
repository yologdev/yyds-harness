# YOUR TRAJECTORY

Last computed: 2026-06-14T04:15Z. Day 106. Window: last 10 sessions / 14 days.

## Recent session outcomes (newest 6 of 10)
day-105 (2026-06-13 17:45:38): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-105 (2026-06-13 10:44:51): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_seed_contradicted=1
day-105 (2026-06-13 04:23:50): tasks 1/3 ⚠️ — 1/3 strict verified; task states: reverted_protected_file_edits=1, reverted_scope_mismatch=1
day-104 (2026-06-12 18:21:44): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_seed_contradicted=1
day-104 (2026-06-12 12:12:45): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-104 (2026-06-12 04:27:18): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Graph-derived next-task pressure
- Close yyds state and model lifecycle gaps (deepseek_model_call_incomplete_count=1): Lifecycle causes: model_incomplete/open_after_command=1; state_incomp...
- Reduce successful-task turn overhead (max_task_turn_count=26): A verified task still used many turns, suggesting discovery or verifi...

## GitHub Actions log feedback
latest score=0.9063 confidence=1.0 recurring_failures=1 state_capture=1.0
Corrected top lessons for next run:
- DeepSeek model call lifecycle was incomplete: model_incomplete/open_after_command=1 -> close model-call lifecycle events on stream errors, timeouts, and abnormal completions
- state run lifecycle was incomplete: state_incomplete/open_after_cache_metrics=1, state_incomplete/open_after_command=1 -> emit RunCompleted events for every started run, including timeout and API-error exits
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 3x fatal: no pattern given
- 3x │ command timed out after 120s
- 2x [2×] error: test failed, to rerun pass `--lib`

## Structured state snapshot
claims: 277/369 proven; 92 non-proven (missing=69, observed=23); 11 recent; recent non-proven claims: model_lifecycle=5 missing, run_lifecycle=5 missing, assessment_artifact=1 observed
- lifecycle gaps: state_incomplete=2, model_incomplete=1
- lifecycle causes: model_incomplete/open_after_command=1, state_incomplete/open_after_cache_metrics=1, state_incomplete/open_after_command=1
- lifecycle aggregate: observed=32/41, unhealthy=21, run_incomplete=43, model_incomplete=24
- recent task issues: reverted_seed_contradicted=2, reverted_no_edit=1, reverted_protected_file_edits=1
- recent assessment artifacts: missing_with_diagnostic=1
- recent tool failures: unrecovered=7/10, failed_commands=7
- recent action evidence: state_only_failed_tools=8, transcript_only_failed_tools=2
- historical unrecovered tool failures: search_regex_error=57 addressed, search_binary_match=19 addressed, missing_file_read=11
... (truncated to fit token budget)
