# YOUR TRAJECTORY

Last computed: 2026-06-17T16:53Z. Day 109. Window: last 10 sessions / 14 days.
_Snapshot age: 256m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-109 (2026-06-17 12:37:13): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_no_edit=1
day-109 (2026-06-17 06:50:37): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_no_edit=1
day-109 (2026-06-17 04:37:35): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-108 (2026-06-16 21:44:09): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-108 (2026-06-16 18:04:37): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-108 (2026-06-16 17:17:37): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unverified=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-109-20260617T121744Z: classification=actionable, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=0.0, task_verification_rate=0.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Force reverted tasks to leave concrete evidence (task_no_edit_revert_count=1): Implementation tasks reverted without touching files; require an earl...
- Raise verified task success rate (task_success_rate=0.0): Dominant task failure: task_no_edit_revert_count=1 (reverted tasks wi...
- Require strict verifier evidence for tasks (task_verification_rate=0.0): Task verification rate was below complete without a counted evaluator...
- Verify readable paths before file reads (failed_tool_summary.read_error=2): verify paths with rg --files and prefer module or symbol discovery wh...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=3): Recent transcripts contained failed tool actions absent from state ev...

## GitHub Actions log feedback
latest score=0.7531 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- implementation tasks reverted without edits -> force implementation agents to either make an early scoped edit, write an obsolete note, or fail with a concrete blocker

## Structured state snapshot
claims: 504/621 proven; 117 non-proven (missing=88, observed=29); 3 recent; recent non-proven claims: run_lifecycle=2 missing, model_lifecycle=1 observed
- lifecycle aggregate: observed=60/69, unhealthy=37, run_incomplete=108, model_incomplete=53
- recent task issues: reverted_no_edit=3
- recent task expected evidence: task_02=Task lineage links src/commands_state.rs change to this task Future self-tests show `state; task_01=Future dashboard sessions show selected task artifacts instead of an empty implementation 
- recent tool failures: unrecovered=6/12, failed_commands=9
... (truncated to fit token budget)
