# YOUR TRAJECTORY

Last computed: 2026-07-03T17:31Z. Day 125. Window: last 10 sessions / 14 days.
_Snapshot age: 370m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-125 (2026-07-03 11:20:51): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-125 (2026-07-03 03:43:42): tasks 0/0 • — no tasks attempted
day-124 (2026-07-02 18:29:34): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unlanded_source_edits=1
day-124 (2026-07-02 11:50:30): tasks 0/2 ⚠️ — 0/2 strict verified; task states: reverted_unlanded_source_edits=2
day-124 (2026-07-02 04:28:34): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_unlanded_source_edits=1
day-123 (2026-07-01 18:20:15): tasks 0/0 • — no tasks attempted
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-125-20260703T103707Z: classification=actionable, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=2, tasks_attempted=2, task_success_rate=0.5, task_verification_rate=0.5, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 0.5
- primary fitness: task_success_rate=0.5, task_verification_rate=0.5
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Force analysis-only attempts into action (task_analysis_only_attempt_count=3): Implementation ended without file progress or terminal evidence; retr...
- Raise verified task success rate (task_success_rate=0.5): Dominant task failure: task_analysis_only_attempt_count=3 (analysis-o...
- Require strict verifier evidence for tasks (task_verification_rate=0.5): Task verification rate was below complete without a counted evaluator...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=14): prefer bounded commands with explicit paths and inspect exit output b...
- Close yyds state and model lifecycle gaps (state_run_incomplete_count=1): Lifecycle causes: state_incomplete/open_after_SessionStarted=1; gaps:...

## GitHub Actions log feedback
latest score=0.7406 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.5 task_spec_quality_score=1.0
Corrected top lessons for next run:
- implementation tasks reverted without edits -> force implementation agents to either make an early scoped edit, write an obsolete note, or fail with a concrete blocker
- state run lifecycle was incomplete: state_incomplete/open_after_SessionStarted=1 -> emit RunCompleted events for every started run, including timeout and API-error exits
... (truncated to fit token budget)
