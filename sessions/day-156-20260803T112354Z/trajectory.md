# YOUR TRAJECTORY

Last computed: 2026-08-03T11:27Z. Day 156. Window: last 10 sessions / 14 days.
_Snapshot age: 371m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-156 (2026-08-03 05:16:01): tasks 1/1 ⚠️ — 1/2 strict verified; 1 no touched files; 1 no passing verifier
day-156 (2026-08-03 04:50:41): tasks 0/2 ⚠️ — 0/2 strict verified; task states: reverted_unlanded_source_edits=2
day-155 (2026-08-02 17:50:21): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_unlanded_source_edits=1
day-155 (2026-08-02 10:43:01): tasks 0/0 • — no tasks attempted
day-155 (2026-08-02 04:56:57): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unlanded_source_edits=1
day-154 (2026-08-01 18:25:06): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unlanded_source_edits=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-156-20260803T041645Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 1.0
- primary fitness: task_success_rate=1.0, task_verification_rate=1.0
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Close yyds state and model lifecycle gaps (deepseek_model_call_unmatched_completed_count=42): Lifecycle causes: model_incomplete/model_completion_without_start=8;...
- Preserve assessment artifacts (assessment_artifact_missing_count=1): Assessment evidence exists but tasks/assessment.md was not preserved,...
- Break recurring log failure fingerprints (recurring_failure_count=1): GitHub/action log feedback repeated failure fingerprints across sessi...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=5): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile state-only tool failures (state_only_failed_tool_count=20): State events contained failed tool actions without matching transcrip...

## GitHub Actions log feedback
latest score=0.7825 confidence=1.0 recurring_failures=1 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- state run lifecycle was incomplete: state_incomplete/open_after_tool_call=1 -> emit RunCompleted events for every started run, including timeout and API-error exits
... (truncated to fit token budget)
