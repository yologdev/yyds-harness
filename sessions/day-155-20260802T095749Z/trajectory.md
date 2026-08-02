# YOUR TRAJECTORY

Last computed: 2026-08-02T10:01Z. Day 155. Window: last 10 sessions / 14 days.
_Snapshot age: 304m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-155 (2026-08-02 04:56:57): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unlanded_source_edits=1
day-154 (2026-08-01 18:25:06): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unlanded_source_edits=1
day-154 (2026-08-01 11:59:38): tasks 0/0 • — no tasks attempted
day-154 (2026-08-01 11:36:01): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-154 (2026-08-01 03:35:42): tasks 0/0 • — no tasks attempted
day-153 (2026-07-31 19:17:59): tasks 2/2 ⚠️ — 1/2 strict verified; raw outcome 2/2; task states: no_git_visible_changes=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-155-20260802T025017Z: classification=actionable, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=2, tasks_attempted=2, task_success_rate=0.5, task_verification_rate=0.5, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 0.5
- primary fitness: task_success_rate=0.5, task_verification_rate=0.5
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Raise verified task success rate (task_success_rate=0.5): Dominant task failure: task_unlanded_source_count=1 (source edits not...
- Bound evaluator checks so verdicts are not skipped (evaluator_unverified_count=1): Some task evals were unverified or timed out.
- Make source-edit outcomes land or explain reverts (task_unlanded_source_count=1): A task touched source files without a landed source commit.
- Break recurring log failure fingerprints (recurring_failure_count=1): GitHub/action log feedback repeated failure fingerprints across sessi...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=5): prefer bounded commands with explicit paths and inspect exit output b...

## GitHub Actions log feedback
latest score=0.5325 confidence=1.0 recurring_failures=1 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.5 task_spec_quality_score=0.85
Corrected top lessons for next run:
- tasks lacked strict verifier evidence -> require bounded verifier evidence before counting task success
- state run lifecycle was incomplete: state_incomplete/open_after_SessionStarted=1 -> emit RunCompleted events for every started run, including timeout and API-error exits
- task source edits were not landed in source commits -> verify task source edits are committed before marking task completion
... (truncated to fit token budget)
