# YOUR TRAJECTORY

Last computed: 2026-06-19T12:11Z. Day 111. Window: last 10 sessions / 14 days.
_Snapshot age: 436m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-111 (2026-06-19 04:55:14): tasks 1/3 ⚠️ — 1/3 strict verified; task states: reverted_no_edit=2
day-110 (2026-06-18 23:35:21): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-110 (2026-06-18 19:43:45): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-110 (2026-06-18 18:44:48): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_no_edit=1
day-110 (2026-06-18 12:16:56): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_no_edit=1
day-110 (2026-06-18 04:51:02): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-111-20260619T042436Z: classification=actionable, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=3, tasks_attempted=3, task_success_rate=0.3333333333333333, task_verification_rate=0.3333333333333333, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Force reverted tasks to leave concrete evidence (task_no_edit_revert_count=2): Implementation tasks reverted without touching files; require an earl...
- Raise verified task success rate (task_success_rate=0.3333333333333333): Dominant task failure: task_no_edit_revert_count=2 (reverted tasks wi...
- Require strict verifier evidence for tasks (task_verification_rate=0.3333333333333333): Task verification rate was below complete without a counted evaluator...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=6): prefer bounded commands with explicit paths and inspect exit output b...
- Close yyds state and model lifecycle gaps (state_run_incomplete_count=1): Lifecycle causes: state_incomplete/open_after_SessionStarted=1; gaps:...

## GitHub Actions log feedback
latest score=0.6458 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.3333333333333333 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- file-read evidence contained path or access errors -> verify paths with rg --files and prefer module or symbol discovery when exact files are uncertain
- implementation tasks reverted without edits -> force implementation agents to either make an early scoped edit, write an obsolete note, or fail with a concrete blocker

## Structured state snapshot
claims: 590/711 proven; 121 non-proven (missing=91, observed=30); 2 recent; recent non-proven claims: run_lifecycle=2 missing
- lifecycle gaps: state_incomplete=1
... (truncated to fit token budget)
