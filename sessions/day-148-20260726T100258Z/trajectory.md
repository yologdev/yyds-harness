# YOUR TRAJECTORY

Last computed: 2026-07-26T10:06Z. Day 148. Window: last 10 sessions / 14 days.
_Snapshot age: 285m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-148 (2026-07-26 05:20:31): tasks 1/3 ⚠️ — 1/3 strict verified; task states: obsolete_already_satisfied=1, reverted_unverified=1
day-147 (2026-07-25 17:44:39): tasks 0/0 • — no tasks attempted
day-147 (2026-07-25 10:30:57): tasks 0/0 • — no tasks attempted
day-147 (2026-07-25 03:26:39): tasks 0/0 • — no tasks attempted
day-146 (2026-07-24 20:01:56): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-146 (2026-07-24 19:17:06): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-148-20260726T025059Z: classification=actionable, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=3, tasks_attempted=3, task_success_rate=0.3333333333333333, task_verification_rate=0.3333333333333333, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 0.3333
- primary fitness: task_success_rate=0.3333333333333333, task_verification_rate=0.3333333333333333
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Raise verified task success rate (task_success_rate=0.3333333333333333): Dominant task failure: evaluator_unverified_count=1 (unverified task...
- Bound evaluator checks so verdicts are not skipped (evaluator_unverified_count=1): Some task evals were unverified or timed out.
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=10): prefer bounded commands with explicit paths and inspect exit output b...
- Replace stale or already-satisfied tasks (task_obsolete_count=1): Implementation marked selected tasks obsolete or already satisfied; p...
- Close yyds state and model lifecycle gaps (deepseek_model_call_unmatched_completed_count=308): Lifecycle causes: model_abnormal/model_completion_without_start=8; st...

## GitHub Actions log feedback
latest score=0.5458 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.3333333333333333 task_spec_quality_score=0.7
Corrected top lessons for next run:
- failed tool actions were recovered from transcripts -> inspect failed tool calls and add prompt/tool guards for the dominant failure class
- tasks lacked strict verifier evidence -> require bounded verifier evidence before counting task success
- state run lifecycle was incomplete: state_incomplete/open_after_RunStarted=1 -> emit RunCompleted events for every started run, including timeout and API-error exits

... (truncated to fit token budget)
