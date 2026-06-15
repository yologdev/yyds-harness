# YOUR TRAJECTORY

Last computed: 2026-06-15T08:55Z. Day 107. Window: last 10 sessions / 14 days.

## Recent session outcomes (newest 6 of 10)
day-107 (2026-06-15 04:42:50): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-107 (2026-06-15 03:21:18): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-106 (2026-06-14 23:12:32): tasks 0/0 • — no tasks attempted
day-106 (2026-06-14 22:53:06): tasks 0/0 • — no tasks attempted
day-106 (2026-06-14 22:01:20): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_seed_contradicted=1
day-106 (2026-06-14 17:30:14): tasks 0/0 • — no tasks attempted
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-107-20260615T042332Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Close yyds state and model lifecycle gaps (deepseek_model_call_unmatched_completed_count=1): Lifecycle causes: state_incomplete/open_after_SessionStarted=1; state...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=2): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile state-only tool failures (state_only_failed_tool_count=10): State events contained failed tool actions without matching transcrip...
- Ignore prose-only DeepSeek cache ratios (deepseek_cache_ratio_unverified_count=1): DeepSeek cache ratios were reported without token-backed cache metric...

## GitHub Actions log feedback
latest score=1.0 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- state run lifecycle was incomplete: state_incomplete/open_after_SessionStarted=1 -> emit RunCompleted events for every started run, including timeout and API-error exits
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 2x │ command timed out after 60s
- 2x fatal: no pattern given

## Structured state snapshot
claims: 342/441 proven; 99 non-proven (missing=75, observed=24); 6 recent; recent non-proven claims: run_lifecycle=3 missing, model_lifecycle=2 missing, assessment_artifact=1 observed
- lifecycle gaps: state_incomplete=1, state_unmatched_non_validation=1, model_unmatched_completed=1
- lifecycle causes: model_unmatched/completion_without_run_start=1, state_incomplete/open_after_SessionStarted=1, state_unmatched/completion_without_run_start=1
- lifecycle aggregate: observed=40/49, unhealthy=25, run_incomplete=46, model_incomplete=24
- recent task issues: reverted_seed_contradicted=1
... (truncated to fit token budget)
