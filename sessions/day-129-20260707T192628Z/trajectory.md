# YOUR TRAJECTORY

Last computed: 2026-07-07T19:29Z. Day 129. Window: last 10 sessions / 14 days.
_Snapshot age: 6m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-129 (2026-07-07 19:22:57): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-129 (2026-07-07 13:07:04): tasks 0/1 ⚠️ — 0/2 strict verified; task states: reverted_unverified=1, scope_mismatch=1
day-129 (2026-07-07 05:57:38): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-129 (2026-07-07 05:02:13): tasks 0/2 ⚠️ — 0/2 strict verified; task states: reverted_no_edit=1, reverted_unlanded_source_edits=1
day-128 (2026-07-06 19:17:29): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-128 (2026-07-06 13:17:53): tasks 1/3 ⚠️ — 1/3 strict verified; task states: reverted_no_edit=2
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-129-20260707T180116Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=2, tasks_attempted=2, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 1.0
- primary fitness: task_success_rate=1.0, task_verification_rate=1.0
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Close yyds state and model lifecycle gaps (deepseek_model_call_abnormal_completed_count=1): Lifecycle causes: state_unmatched/run_error_without_start=7; model_ab...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=6): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=2): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=30): State events contained failed tool actions without matching transcrip...

## GitHub Actions log feedback
latest score=0.6125 confidence=1.0 recurring_failures=2 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 5x error: test failed, to rerun pass `--lib`
- 3x │ command timed out after 30s
- 2x │ command timed out after 120s
... (truncated to fit token budget)
