# YOUR TRAJECTORY

Last computed: 2026-06-23T03:43Z. Day 115. Window: last 10 sessions / 14 days.
_Snapshot age: 240m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-114 (2026-06-22 23:43:05): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-114 (2026-06-22 19:51:52): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-114 (2026-06-22 15:53:07): tasks 1/2 ⚠️ — 1/2 strict verified; task states: obsolete_already_satisfied=1
day-114 (2026-06-22 14:02:07): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-114 (2026-06-22 13:01:24): tasks 0/1 ⚠️ — 0/1 strict verified; task states: obsolete_already_satisfied=1
day-114 (2026-06-22 09:28:16): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-114-20260622T230633Z: classification=actionable, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=2, tasks_attempted=2, task_success_rate=0.5, task_verification_rate=0.5, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Force analysis-only attempts into action (task_analysis_only_attempt_count=1): Implementation ended without file progress or terminal evidence; retr...
- Raise verified task success rate (task_success_rate=0.5): Dominant task failure: task_analysis_only_attempt_count=1 (analysis-o...
- Require strict verifier evidence for tasks (task_verification_rate=0.5): Task verification rate was below complete without a counted evaluator...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=4): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=2): Recent transcripts contained failed tool actions absent from state ev...

## GitHub Actions log feedback
latest score=0.8031 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.5 task_spec_quality_score=1.0
Corrected top lessons for next run:
- failed tool actions were recovered from transcripts -> inspect failed tool calls and add prompt/tool guards for the dominant failure class
- implementation tasks reverted without edits -> force implementation agents to either make an early scoped edit, write an obsolete note, or fail with a concrete blocker

## Structured state snapshot
claims: 727/855 proven; 128 non-proven (missing=96, observed=32); recent non-proven claims: run_lifecycle=52 missing, model_lifecycle=44 missing, assessment_artifact=25 observed
- lifecycle aggregate: observed=86/95, unhealthy=45, run_incomplete=117, model_incomplete=54
- recent task issues: reverted_no_edit=2
... (truncated to fit token budget)
