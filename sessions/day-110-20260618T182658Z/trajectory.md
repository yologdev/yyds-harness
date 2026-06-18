# YOUR TRAJECTORY

Last computed: 2026-06-18T18:30Z. Day 110. Window: last 10 sessions / 14 days.
_Snapshot age: 374m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-110 (2026-06-18 12:16:56): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_no_edit=1
day-110 (2026-06-18 04:51:02): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
day-109 (2026-06-17 23:44:22): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
day-109 (2026-06-17 20:46:55): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-109 (2026-06-17 18:46:07): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-109 (2026-06-17 17:25:01): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-110-20260618T115133Z: classification=actionable, can_drive_evolution=true
- warning: implementation terminal marker missing on 1 attempt(s); mechanical task proof exists
- evidence: provider_error_count=0, selected_task_count=3, tasks_attempted=3, task_success_rate=0.6666666666666666, task_verification_rate=0.6666666666666666, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Force reverted tasks to leave concrete evidence (task_no_edit_revert_count=1): Implementation tasks reverted without touching files; require an earl...
- Raise verified task success rate (task_success_rate=0.6666666666666666): Dominant task failure: task_no_edit_revert_count=1 (reverted tasks wi...
- Require strict verifier evidence for tasks (task_verification_rate=0.6666666666666666): Task verification rate was below complete without a counted evaluator...
- Reconcile state-only tool failures (state_only_failed_tool_count=11): State events contained failed tool actions without matching transcrip...
- Emit terminal markers after verified commits (task_terminal_marker_missing_attempt_count=1): Implementation landed mechanical proof but omitted the exact TASK_TER...

## GitHub Actions log feedback
latest score=0.8276 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.6666666666666666 task_spec_quality_score=1.0
Corrected top lessons for next run:
- implementation tasks reverted without edits -> force implementation agents to either make an early scoped edit, write an obsolete note, or fail with a concrete blocker

## Structured state snapshot
claims: 556/675 proven; 119 non-proven (missing=89, observed=30); 2 recent; recent non-proven claims: assessment_artifact=1 observed, run_lifecycle=1 missing
- lifecycle aggregate: observed=66/75, unhealthy=38, run_incomplete=109, model_incomplete=53
- recent task issues: reverted_no_edit=2
... (truncated to fit token budget)
