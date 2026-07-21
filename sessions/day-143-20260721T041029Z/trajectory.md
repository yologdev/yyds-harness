# YOUR TRAJECTORY

Last computed: 2026-07-21T04:23Z. Day 143. Window: last 10 sessions / 14 days.
_Snapshot age: 9m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-143 (2026-07-21 04:13:43): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_unlanded_source_edits=1
day-142 (2026-07-20 12:55:57): tasks 0/0 • — no tasks attempted
day-142 (2026-07-20 12:54:04): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unlanded_source_edits=1
day-142 (2026-07-20 04:23:09): tasks 0/1 ⚠️ — 0/2 strict verified; task states: reverted_unlanded_source_edits=1
day-141 (2026-07-19 18:54:15): tasks 0/0 • — no tasks attempted
day-141 (2026-07-19 18:49:22): tasks 0/2 ⚠️ — 0/2 strict verified; task states: reverted_unlanded_source_edits=2
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-143-20260721T024517Z: classification=actionable, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=0.0, task_verification_rate=0.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 0.0
- primary fitness: task_success_rate=0.0, task_verification_rate=0.0
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Raise verified task success rate (outcome_task_success_rate=0.0): Dominant task failure: task_unlanded_source_count=1 (source edits not...
- Bound evaluator checks so verdicts are not skipped (evaluator_unverified_count=1): Some task evals were unverified or timed out.
- Make source-edit outcomes land or explain reverts (task_unlanded_source_count=1): A task touched source files without a landed source commit.
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=20): prefer bounded commands with explicit paths and inspect exit output b...
- Close yyds state and model lifecycle gaps (deepseek_model_call_unmatched_completed_count=306): Lifecycle causes: model_abnormal/model_completion_without_start=8; st...

## GitHub Actions log feedback
latest score=0.6625 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- tasks lacked strict verifier evidence -> require bounded verifier evidence before counting task success
- task source edits were not landed in source commits -> verify task source edits are committed before marking task completion
... (truncated to fit token budget)
