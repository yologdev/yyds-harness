# YOUR TRAJECTORY

Last computed: 2026-06-16T12:57Z. Day 108. Window: last 10 sessions / 14 days.
_Snapshot age: 199m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-108 (2026-06-16 09:37:47): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-108 (2026-06-16 04:58:24): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-108 (2026-06-16 01:23:13): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-107 (2026-06-15 22:54:25): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-107 (2026-06-15 21:35:21): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_seed_contradicted=1
day-107 (2026-06-15 21:13:04): tasks 0/2 ⚠️ — 0/2 strict verified; task states: reverted_unlanded_source_edits=2
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-108-20260616T090120Z: classification=verified_success, can_drive_evolution=true
- warning: implementation terminal marker missing on 1 attempt(s); mechanical task proof exists
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=3): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=19): State events contained failed tool actions without matching transcrip...
- Recover failed tool actions before scoring (tool_error_count=2): Failed tool actions were present in session evidence; inspect the dom...
- Harden search commands and pattern escaping (search_error_count=1): Search/grep errors created avoidable evolution friction.
- Emit terminal markers after verified commits (task_terminal_marker_missing_attempt_count=1): Implementation landed mechanical proof but omitted the exact TASK_TER...

## GitHub Actions log feedback
latest score=0.8516 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 5x error: test failed, to rerun pass `--lib`
- 4x thread 'state::tests::run_completion_guard_reports_error_on_panic' (<n>) panicked at src/s
- 3x thread 'empty_piped_stdin_exits_quickly' (<n>) panicked at tests/integration.rs:n:n:

## Structured state snapshot
claims: 430/540 proven; 110 non-proven (missing=84, observed=26); 3 recent; recent non-proven claims: run_lifecycle=3 missing
... (truncated to fit token budget)
