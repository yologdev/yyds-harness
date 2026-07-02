# YOUR TRAJECTORY

Last computed: 2026-07-02T10:45Z. Day 124. Window: last 10 sessions / 14 days.
_Snapshot age: 376m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-124 (2026-07-02 04:28:34): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_unlanded_source_edits=1
day-123 (2026-07-01 18:20:15): tasks 0/0 • — no tasks attempted
day-123 (2026-07-01 12:19:28): tasks 0/2 ⚠️ — 0/2 strict verified; task states: reverted_unlanded_source_edits=2
day-123 (2026-07-01 04:43:02): tasks 0/2 ⚠️ — 0/2 strict verified; task states: reverted_no_edit=1, reverted_unlanded_source_edits=1
day-122 (2026-06-30 18:14:53): tasks 0/0 • — no tasks attempted
day-122 (2026-06-30 11:50:23): tasks 1/3 ⚠️ — 1/3 strict verified; task states: reverted_unlanded_source_edits=2
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-124-20260702T034038Z: classification=actionable, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=3, tasks_attempted=3, task_success_rate=0.6666666666666666, task_verification_rate=0.6666666666666666, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 0.6667
- primary fitness: task_success_rate=0.6666666666666666, task_verification_rate=0.6666666666666666
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Force analysis-only attempts into action (task_analysis_only_attempt_count=1): Implementation ended without file progress or terminal evidence; retr...
- Raise verified task success rate (task_success_rate=0.6666666666666666): Dominant task failure: task_unlanded_source_count=1 (source edits not...
- Make source-edit outcomes land or explain reverts (task_unlanded_source_count=1): A task touched source files without a landed source commit.
- Require strict verifier evidence for tasks (task_verification_rate=0.6666666666666666): Task verification rate was below complete without a counted evaluator...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=5): prefer bounded commands with explicit paths and inspect exit output b...

## GitHub Actions log feedback
latest score=0.6792 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.6666666666666666 task_spec_quality_score=1.0
Corrected top lessons for next run:
- edit failed because the replacement context was ambiguous or absent -> read a tighter surrounding range and use unique old_text context before applying edits
- tasks lacked strict verifier evidence -> require bounded verifier evidence before counting task success
... (truncated to fit token budget)
