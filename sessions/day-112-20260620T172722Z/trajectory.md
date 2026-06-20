# YOUR TRAJECTORY

Last computed: 2026-06-20T17:31Z. Day 112. Window: last 10 sessions / 14 days.
_Snapshot age: 391m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-112 (2026-06-20 10:59:13): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-112 (2026-06-20 04:08:32): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-111 (2026-06-19 18:26:07): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-111 (2026-06-19 12:42:23): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-111 (2026-06-19 04:55:14): tasks 1/3 ⚠️ — 1/3 strict verified; task states: reverted_no_edit=2
day-110 (2026-06-18 23:35:21): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-112-20260620T103310Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=2, tasks_attempted=2, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=12): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=5): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=24): State events contained failed tool actions without matching transcrip...
- Recover failed tool actions before scoring (tool_error_count=3): Failed tool actions were present in session evidence; inspect the dom...
- Harden search commands and pattern escaping (search_error_count=1): Search/grep errors created avoidable evolution friction.

## GitHub Actions log feedback
latest score=0.9219 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 2x │ command timed out after 120s
- 2x error: test failed, to rerun pass `--lib`

## Structured state snapshot
claims: 625/747 proven; 122 non-proven (missing=92, observed=30); 2 recent; recent non-proven claims: run_lifecycle=2 missing
- lifecycle aggregate: observed=74/83, unhealthy=41, run_incomplete=113, model_incomplete=53
- recent task issues: reverted_no_edit=3
... (truncated to fit token budget)
