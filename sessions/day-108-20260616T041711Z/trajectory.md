# YOUR TRAJECTORY

Last computed: 2026-06-16T04:21Z. Day 108. Window: last 10 sessions / 14 days.
_Snapshot age: 177m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-108 (2026-06-16 01:23:13): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-107 (2026-06-15 22:54:25): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-107 (2026-06-15 21:35:21): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_seed_contradicted=1
day-107 (2026-06-15 21:13:04): tasks 0/2 ⚠️ — 0/2 strict verified; task states: reverted_unlanded_source_edits=2
day-107 (2026-06-15 17:28:17): tasks 1/2 ⚠️ — 0/2 strict verified; raw outcome 1/2; task states: reverted_seed_contradicted=1
day-107 (2026-06-15 15:08:03): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-108-20260616T003921Z: classification=verified_success, can_drive_evolution=true
- warning: implementation terminal marker missing on 2 attempt(s); mechanical task proof exists
- evidence: provider_error_count=0, selected_task_count=2, tasks_attempted=2, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Break recurring log failure fingerprints (recurring_failure_count=1): GitHub/action log feedback repeated failure fingerprints across sessi...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=6): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=5): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=35): State events contained failed tool actions without matching transcrip...
- Recover failed tool actions before scoring (tool_error_count=1): Failed tool actions were present in session evidence; inspect the dom...

## GitHub Actions log feedback
latest score=0.845 confidence=1.0 recurring_failures=1 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- failed tool actions were recovered from transcripts -> inspect failed tool calls and add prompt/tool guards for the dominant failure class
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 6x error: test failed, to rerun pass `--lib`
- 5x thread 'state::tests::run_completion_guard_reports_error_on_panic' (<n>) panicked at src/s
- 3x │ command timed out after 180s

## Structured state snapshot
claims: 413/522 proven; 109 non-proven (missing=83, observed=26); 3 recent; recent non-proven claims: run_lifecycle=3 missing
... (truncated to fit token budget)
