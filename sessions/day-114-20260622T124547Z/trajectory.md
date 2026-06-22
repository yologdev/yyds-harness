# YOUR TRAJECTORY

Last computed: 2026-06-22T12:48Z. Day 114. Window: last 10 sessions / 14 days.
_Snapshot age: 200m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-114 (2026-06-22 09:28:16): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-114 (2026-06-22 04:49:15): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-113 (2026-06-21 23:35:29): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unlanded_source_edits=1
day-113 (2026-06-21 18:26:33): tasks 1/3 ⚠️ — 1/3 strict verified; task states: reverted_no_edit=2
day-113 (2026-06-21 11:37:12): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-113 (2026-06-21 04:37:16): tasks 0/1 ⚠️ — 0/1 strict verified; task states: obsolete_already_satisfied=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-114-20260622T084851Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=2, tasks_attempted=2, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Close yyds state and model lifecycle gaps (deepseek_model_call_abnormal_completed_count=1): Lifecycle causes: model_abnormal/model_completion_without_start=1; st...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=6): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=1): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=36): State events contained failed tool actions without matching transcrip...
- Ignore prose-only DeepSeek cache ratios (deepseek_cache_ratio_unverified_count=1): DeepSeek cache ratios were reported without token-backed cache metric...

## GitHub Actions log feedback
latest score=0.9375 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- state run lifecycle was incomplete: state_incomplete/open_after_SessionStarted=1 -> emit RunCompleted events for every started run, including timeout and API-error exits

## Structured state snapshot
claims: 682/810 proven; 128 non-proven (missing=96, observed=32); 4 recent; recent non-proven claims: model_lifecycle=2 observed, run_lifecycle=2 missing
- lifecycle gaps: state_incomplete=1
- lifecycle causes: state_incomplete/open_after_SessionStarted=1
- lifecycle aggregate: observed=81/90, unhealthy=45, run_incomplete=117, model_incomplete=54
- recent task issues: reverted_no_edit=2, reverted_unlanded_source_edits=1
... (truncated to fit token budget)
