# YOUR TRAJECTORY

Last computed: 2026-06-16T00:43Z. Day 108. Window: last 10 sessions / 14 days.
_Snapshot age: 108m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-107 (2026-06-15 22:54:25): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-107 (2026-06-15 21:35:21): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_seed_contradicted=1
day-107 (2026-06-15 21:13:04): tasks 0/2 ⚠️ — 0/2 strict verified; task states: reverted_unlanded_source_edits=2
day-107 (2026-06-15 17:28:17): tasks 1/2 ⚠️ — 0/2 strict verified; raw outcome 1/2; task states: reverted_seed_contradicted=1
day-107 (2026-06-15 15:08:03): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
day-107 (2026-06-15 13:04:31): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-107-20260615T222443Z: classification=verified_success, can_drive_evolution=true
- warning: implementation terminal marker missing on 1 attempt(s); mechanical task proof exists
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Close yyds state and model lifecycle gaps (state_run_incomplete_count=1): Lifecycle causes: state_incomplete/open_after_SessionStarted=1; gaps:...
- Force analysis-only attempts into action (task_analysis_only_attempt_count=1): Implementation ended without file progress or terminal evidence; retr...
- Break recurring log failure fingerprints (recurring_failure_count=2): GitHub/action log feedback repeated failure fingerprints across sessi...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=22): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=4): Recent transcripts contained failed tool actions absent from state ev...

## GitHub Actions log feedback
latest score=0.8125 confidence=1.0 recurring_failures=2 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- state run lifecycle was incomplete: state_incomplete/open_after_SessionStarted=1 -> emit RunCompleted events for every started run, including timeout and API-error exits
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 5x thread 'state::tests::run_completion_guard_reports_error_on_panic' (<n>) panicked at src/s
- 5x error: test failed, to rerun pass `--lib`
- 3x │ command timed out after 180s
... (truncated to fit token budget)
