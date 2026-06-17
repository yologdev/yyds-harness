# YOUR TRAJECTORY

Last computed: 2026-06-17T18:23Z. Day 109. Window: last 10 sessions / 14 days.
_Snapshot age: 58m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-109 (2026-06-17 17:25:01): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-109 (2026-06-17 12:37:13): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_no_edit=1
day-109 (2026-06-17 06:50:37): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_no_edit=1
day-109 (2026-06-17 04:37:35): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-108 (2026-06-16 21:44:09): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-108 (2026-06-16 18:04:37): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-109-20260617T164949Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Verify readable paths before file reads (failed_tool_summary.read_error=2): verify paths with rg --files and prefer module or symbol discovery wh...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=3): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=10): State events contained failed tool actions without matching transcrip...
- Recover failed tool actions before scoring (tool_error_count=2): Failed tool actions were present in session evidence; inspect the dom...
- Reduce successful-task turn overhead (max_task_turn_count=25): A verified task still used many turns, suggesting discovery or verifi...

## GitHub Actions log feedback
latest score=0.9688 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- file-read evidence contained path or access errors -> verify paths with rg --files and prefer module or symbol discovery when exact files are uncertain
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks

## Structured state snapshot
claims: 513/630 proven; 117 non-proven (missing=88, observed=29); 2 recent; recent non-proven claims: model_lifecycle=1 observed, run_lifecycle=1 missing
- lifecycle aggregate: observed=61/70, unhealthy=37, run_incomplete=108, model_incomplete=53
- recent task issues: reverted_no_edit=3
... (truncated to fit token budget)
