# YOUR TRAJECTORY

Last computed: 2026-07-24T11:48Z. Day 146. Window: last 10 sessions / 14 days.
_Snapshot age: 12m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-146 (2026-07-24 11:35:53): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-146 (2026-07-24 05:13:00): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-146 (2026-07-24 04:19:12): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-145 (2026-07-23 18:27:44): tasks 0/2 ⚠️ — 0/2 strict verified; task states: obsolete_already_satisfied=1, reverted_no_edit=1
day-145 (2026-07-23 10:53:51): tasks 0/0 • — no tasks attempted
day-145 (2026-07-23 03:26:34): tasks 0/0 • — no tasks attempted
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-146-20260724T101847Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 1.0
- primary fitness: task_success_rate=1.0, task_verification_rate=1.0
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Close yyds state and model lifecycle gaps (deepseek_model_call_unmatched_completed_count=154): Lifecycle causes: model_abnormal/model_completion_without_start=8; st...
- Force analysis-only attempts into action (task_analysis_only_attempt_count=1): Implementation ended without file progress or terminal evidence; retr...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=1): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=31): State events contained failed tool actions without matching transcrip...

## GitHub Actions log feedback
latest score=0.7125 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- state run lifecycle was incomplete: state_incomplete/open_after_SessionStarted=1 -> emit RunCompleted events for every started run, including timeout and API-error exits
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 3x │ command timed out after 240s
- 2x evaluator: timed out — failing task because no verifier verdict exists
... (truncated to fit token budget)
