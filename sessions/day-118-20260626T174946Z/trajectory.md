# YOUR TRAJECTORY

Last computed: 2026-06-26T17:53Z. Day 118. Window: last 10 sessions / 14 days.
_Snapshot age: 388m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-118 (2026-06-26 11:24:57): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-118 (2026-06-26 04:28:04): tasks 2/3 ⚠️ — 2/3 strict verified; task states: obsolete_already_satisfied=1
day-117 (2026-06-25 18:52:50): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_no_edit=1
day-117 (2026-06-25 11:05:07): tasks 0/0 • — no tasks attempted
day-117 (2026-06-25 03:57:40): tasks 0/0 • — no tasks attempted
day-117 (2026-06-25 01:12:53): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_unlanded_source_edits=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-118-20260626T105245Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 1.0
- primary fitness: task_success_rate=1.0, task_verification_rate=1.0
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=8): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=1): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=36): State events contained failed tool actions without matching transcrip...
- Recover failed tool actions before scoring (tool_error_count=1): Failed tool actions were present in session evidence; inspect the dom...
- Reduce successful-task turn overhead (max_task_turn_count=33): A verified task still used many turns, suggesting discovery or verifi...

## GitHub Actions log feedback
latest score=0.9688 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 2x │ command timed out after 120s
... (truncated to fit token budget)
