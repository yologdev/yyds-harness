# YOUR TRAJECTORY

Last computed: 2026-07-01T18:01Z. Day 123. Window: last 10 sessions / 14 days.
_Snapshot age: 341m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-123 (2026-07-01 12:19:28): tasks 0/2 ⚠️ — 0/2 strict verified; task states: reverted_unlanded_source_edits=2
day-123 (2026-07-01 04:43:02): tasks 0/2 ⚠️ — 0/2 strict verified; task states: reverted_no_edit=1, reverted_unlanded_source_edits=1
day-122 (2026-06-30 18:14:53): tasks 0/0 • — no tasks attempted
day-122 (2026-06-30 11:50:23): tasks 1/3 ⚠️ — 1/3 strict verified; task states: reverted_unlanded_source_edits=2
day-122 (2026-06-30 04:45:31): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unlanded_source_edits=1
day-121 (2026-06-29 18:36:45): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-123-20260701T112456Z: classification=actionable, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=2, tasks_attempted=2, task_success_rate=0.0, task_verification_rate=0.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 0.0
- primary fitness: task_success_rate=0.0, task_verification_rate=0.0
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Raise verified task success rate (task_success_rate=0.0): Dominant task failure: task_unlanded_source_count=2 (source edits not...
- Bound evaluator checks so verdicts are not skipped (evaluator_unverified_count=2): Some task evals were unverified or timed out.
- Make source-edit outcomes land or explain reverts (task_unlanded_source_count=2): A task touched source files without a landed source commit.
- Break recurring log failure fingerprints (recurring_failure_count=1): GitHub/action log feedback repeated failure fingerprints across sessi...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=12): prefer bounded commands with explicit paths and inspect exit output b...

## GitHub Actions log feedback
latest score=0.6125 confidence=1.0 recurring_failures=1 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- tasks lacked strict verifier evidence -> require bounded verifier evidence before counting task success
- task source edits were not landed in source commits -> verify task source edits are committed before marking task completion
... (truncated to fit token budget)
