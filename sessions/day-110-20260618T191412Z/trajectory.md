# YOUR TRAJECTORY

Last computed: 2026-06-18T19:18Z. Day 110. Window: last 10 sessions / 14 days.
_Snapshot age: 33m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-110 (2026-06-18 18:44:48): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_no_edit=1
day-110 (2026-06-18 12:16:56): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_no_edit=1
day-110 (2026-06-18 04:51:02): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
day-109 (2026-06-17 23:44:22): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
day-109 (2026-06-17 20:46:55): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-109 (2026-06-17 18:46:07): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-110-20260618T182658Z: classification=actionable, can_drive_evolution=true
- warning: task implementation terminal evidence incomplete for 1 task artifact(s)
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=0.0, task_verification_rate=0.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Force reverted tasks to leave concrete evidence (task_no_edit_revert_count=1): Implementation tasks reverted without touching files; require an earl...
- Raise verified task success rate (task_success_rate=0.0): Dominant task failure: task_no_edit_revert_count=1 (reverted tasks wi...
- Require strict verifier evidence for tasks (task_verification_rate=0.0): Task verification rate was below complete without a counted evaluator...
- Require terminal task evidence before completion (task_incomplete_terminal_count=1): Implementation exited cleanly without TASK_TERMINAL_EVIDENCE or mecha...
- Close yyds state and model lifecycle gaps (state_run_incomplete_count=2): Lifecycle causes: state_incomplete/open_after_SessionStarted=2; gaps:...

## GitHub Actions log feedback
latest score=0.6516 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- implementation tasks reverted without edits -> force implementation agents to either make an early scoped edit, write an obsolete note, or fail with a concrete blocker
- state run lifecycle was incomplete: state_incomplete/open_after_SessionStarted=2 -> emit RunCompleted events for every started run, including timeout and API-error exits

## Structured state snapshot
claims: 564/684 proven; 120 non-proven (missing=90, observed=30); 3 recent; recent non-proven claims: run_lifecycle=2 missing, assessment_artifact=1 observed
- lifecycle gaps: state_incomplete=2
- lifecycle causes: state_incomplete/open_after_SessionStarted=2
- lifecycle aggregate: observed=67/76, unhealthy=39, run_incomplete=111, model_incomplete=53
... (truncated to fit token budget)
