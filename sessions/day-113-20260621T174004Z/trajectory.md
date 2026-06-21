# YOUR TRAJECTORY

Last computed: 2026-06-21T17:44Z. Day 113. Window: last 10 sessions / 14 days.
_Snapshot age: 366m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-113 (2026-06-21 11:37:12): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-113 (2026-06-21 04:37:16): tasks 0/1 ⚠️ — 0/1 strict verified; task states: obsolete_already_satisfied=1
day-112 (2026-06-20 18:04:15): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
day-112 (2026-06-20 10:59:13): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-112 (2026-06-20 04:08:32): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-111 (2026-06-19 18:26:07): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-113-20260621T111720Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=4): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=2): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=27): State events contained failed tool actions without matching transcrip...
- Recover failed tool actions before scoring (tool_error_count=1): Failed tool actions were present in session evidence; inspect the dom...

## GitHub Actions log feedback
latest score=0.9844 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- failed tool actions were recovered from transcripts -> inspect failed tool calls and add prompt/tool guards for the dominant failure class
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 2x │ command timed out after 120s
- 2x error: test failed, to rerun pass `--lib`

## Structured state snapshot
claims: 650/774 proven; 124 non-proven (missing=94, observed=30); 2 recent; recent non-proven claims: model_lifecycle=1 missing, run_lifecycle=1 missing
- lifecycle aggregate: observed=77/86, unhealthy=42, run_incomplete=114, model_incomplete=54
- recent tool failures: unrecovered=8/29, failed_commands=26
- recent action evidence: state_only_failed_tools=27, transcript_only_failed_tools=2
- gnome evidence audit: adjusted=3314 across 86 session(s), top_sources=log_feedback=2592, task_artifacts=202, state_lifecycle.runs=192; reconciliation_not_raw_bug_count
... (truncated to fit token budget)
