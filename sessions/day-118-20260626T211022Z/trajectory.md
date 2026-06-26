# YOUR TRAJECTORY

Last computed: 2026-06-26T21:14Z. Day 118. Window: last 10 sessions / 14 days.
_Snapshot age: 161m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-118 (2026-06-26 18:32:20): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_unlanded_source_edits=1
day-118 (2026-06-26 11:24:57): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-118 (2026-06-26 04:28:04): tasks 2/3 ⚠️ — 2/3 strict verified; task states: obsolete_already_satisfied=1
day-117 (2026-06-25 18:52:50): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_no_edit=1
day-117 (2026-06-25 11:05:07): tasks 0/0 • — no tasks attempted
day-117 (2026-06-25 03:57:40): tasks 0/0 • — no tasks attempted
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-118-20260626T174946Z: classification=actionable, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=3, tasks_attempted=3, task_success_rate=0.6666666666666666, task_verification_rate=0.6666666666666666, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 0.6667
- primary fitness: task_success_rate=0.6666666666666666, task_verification_rate=0.6666666666666666
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Raise verified task success rate (task_success_rate=0.6666666666666666): Dominant task failure: task_unlanded_source_count=1 (source edits not...
- Bound evaluator checks so verdicts are not skipped (evaluator_unverified_count=1): Some task evals were unverified or timed out.
- Make source-edit outcomes land or explain reverts (task_unlanded_source_count=1): A task touched source files without a landed source commit.
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=10): prefer bounded commands with explicit paths and inspect exit output b...
- Make evaluator timeouts resumable or cheaper (evaluator_timeout_count=1): Evaluator timeout friction still appears in action logs.

## GitHub Actions log feedback
latest score=0.6948 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.6666666666666666 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- tasks lacked strict verifier evidence -> require bounded verifier evidence before counting task success
- task source edits were not landed in source commits -> verify task source edits are committed before marking task completion
... (truncated to fit token budget)
