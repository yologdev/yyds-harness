# YOUR TRAJECTORY

Last computed: 2026-06-16T16:34Z. Day 108. Window: last 10 sessions / 14 days.
_Snapshot age: 68m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-108 (2026-06-16 15:25:46): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-108 (2026-06-16 14:30:29): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_unlanded_source_edits=1
day-108 (2026-06-16 13:28:27): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-108 (2026-06-16 09:37:47): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-108 (2026-06-16 04:58:24): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-108 (2026-06-16 01:23:13): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-108-20260616T145533Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=3): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=1): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=16): State events contained failed tool actions without matching transcrip...
- Recover failed tool actions before scoring (tool_error_count=1): Failed tool actions were present in session evidence; inspect the dom...
- Reduce successful-task turn overhead (max_task_turn_count=27): A verified task still used many turns, suggesting discovery or verifi...

## GitHub Actions log feedback
latest score=0.9531 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- failed tool actions were recovered from transcripts -> inspect failed tool calls and add prompt/tool guards for the dominant failure class
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 3x thread 'empty_piped_stdin_exits_quickly' (<n>) panicked at tests/integration.rs:n:n:
- 3x error: test failed, to rerun pass `--test integration`
- 2x │ search error: grep: src/main.rs: no such file or directory

## Structured state snapshot
claims: 455/567 proven; 112 non-proven (missing=85, observed=27); 3 recent; recent non-proven claims: run_lifecycle=2 missing, model_lifecycle=1 observed
- lifecycle aggregate: observed=54/63, unhealthy=34, run_incomplete=106, model_incomplete=53
- recent task issues: reverted_unlanded_source_edits=1
... (truncated to fit token budget)
