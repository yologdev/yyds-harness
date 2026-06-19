# YOUR TRAJECTORY

Last computed: 2026-06-19T18:03Z. Day 111. Window: last 10 sessions / 14 days.
_Snapshot age: 321m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-111 (2026-06-19 12:42:23): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-111 (2026-06-19 04:55:14): tasks 1/3 ⚠️ — 1/3 strict verified; task states: reverted_no_edit=2
day-110 (2026-06-18 23:35:21): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-110 (2026-06-18 19:43:45): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-110 (2026-06-18 18:44:48): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_no_edit=1
day-110 (2026-06-18 12:16:56): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_no_edit=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-111-20260619T120730Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Close yyds state and model lifecycle gaps (state_run_incomplete_count=1): Lifecycle causes: state_incomplete/open_after_SessionStarted=1; gaps:...
- Break recurring log failure fingerprints (recurring_failure_count=1): GitHub/action log feedback repeated failure fingerprints across sessi...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=7): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=3): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=17): State events contained failed tool actions without matching transcrip...

## GitHub Actions log feedback
latest score=0.8138 confidence=1.0 recurring_failures=1 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- state run lifecycle was incomplete: state_incomplete/open_after_SessionStarted=1 -> emit RunCompleted events for every started run, including timeout and API-error exits
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 2x error: test failed, to rerun pass `--lib`

## Structured state snapshot
claims: 598/720 proven; 122 non-proven (missing=92, observed=30); 3 recent; recent non-proven claims: run_lifecycle=3 missing
- lifecycle gaps: state_incomplete=1
- lifecycle causes: state_incomplete/open_after_SessionStarted=1
... (truncated to fit token budget)
