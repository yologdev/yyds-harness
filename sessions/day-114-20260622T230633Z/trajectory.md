# YOUR TRAJECTORY

Last computed: 2026-06-22T23:10Z. Day 114. Window: last 10 sessions / 14 days.
_Snapshot age: 198m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-114 (2026-06-22 19:51:52): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-114 (2026-06-22 15:53:07): tasks 1/2 ⚠️ — 1/2 strict verified; task states: obsolete_already_satisfied=1
day-114 (2026-06-22 14:02:07): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-114 (2026-06-22 13:01:24): tasks 0/1 ⚠️ — 0/1 strict verified; task states: obsolete_already_satisfied=1
day-114 (2026-06-22 09:28:16): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-114 (2026-06-22 04:49:15): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-114-20260622T192938Z: classification=actionable, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=2, tasks_attempted=2, task_success_rate=0.5, task_verification_rate=0.5, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Force analysis-only attempts into action (task_analysis_only_attempt_count=1): Implementation ended without file progress or terminal evidence; retr...
- Raise verified task success rate (task_success_rate=0.5): Dominant task failure: task_analysis_only_attempt_count=1 (analysis-o...
- Require strict verifier evidence for tasks (task_verification_rate=0.5): Task verification rate was below complete without a counted evaluator...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=4): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=1): Recent transcripts contained failed tool actions absent from state ev...

## GitHub Actions log feedback
latest score=0.8344 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.5 task_spec_quality_score=1.0
Corrected top lessons for next run:
- implementation tasks reverted without edits -> force implementation agents to either make an early scoped edit, write an obsolete note, or fail with a concrete blocker

## Structured state snapshot
claims: 718/846 proven; 128 non-proven (missing=96, observed=32); 2 recent; recent non-proven claims: model_lifecycle=1 observed, run_lifecycle=1 missing
- lifecycle aggregate: observed=85/94, unhealthy=45, run_incomplete=117, model_incomplete=54
- recent task issues: reverted_no_edit=1
- recent task expected evidence: task_02=Future dashboard runs show transcript_only_failed_tool_count=0 when state capture is compl
- recent tool failures: unrecovered=7/34, failed_commands=32
- recent action evidence: state_only_failed_tools=33, transcript_only_failed_tools=1
... (truncated to fit token budget)
