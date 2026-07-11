# YOUR TRAJECTORY

Last computed: 2026-07-11T17:02Z. Day 133. Window: last 10 sessions / 14 days.
_Snapshot age: 334m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-133 (2026-07-11 11:28:00): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-133 (2026-07-11 04:55:33): tasks 0/2 ⚠️ — 0/2 strict verified; task states: obsolete_already_satisfied=1, reverted_no_edit=1
day-133 (2026-07-11 04:41:02): tasks 1/3 ⚠️ — 1/3 strict verified; task states: reverted_no_edit=1, reverted_unlanded_source_edits=1
day-132 (2026-07-10 20:12:24): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_scope_mismatch=1
day-132 (2026-07-10 19:47:25): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
day-132 (2026-07-10 12:05:29): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-133-20260711T093845Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=2, tasks_attempted=2, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 1.0
- primary fitness: task_success_rate=1.0, task_verification_rate=1.0
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Close yyds state and model lifecycle gaps (deepseek_model_call_abnormal_completed_count=2): Lifecycle causes: state_unmatched/run_error_without_start=8; model_ab...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=30): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=1): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=63): State events contained failed tool actions without matching transcrip...
- Recover failed tool actions before scoring (tool_error_count=8): Failed tool actions were present in session evidence; inspect the dom...

## GitHub Actions log feedback
latest score=0.7125 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- file-read evidence contained path or access errors -> verify paths with rg --files and prefer module or symbol discovery when exact files are uncertain
... (truncated to fit token budget)
