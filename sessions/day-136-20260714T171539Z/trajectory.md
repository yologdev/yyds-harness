# YOUR TRAJECTORY

Last computed: 2026-07-14T17:18Z. Day 136. Window: last 10 sessions / 14 days.
_Snapshot age: 373m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-136 (2026-07-14 11:05:01): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-136 (2026-07-14 04:34:32): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unverified=1
day-135 (2026-07-13 18:36:29): tasks 0/0 • — no tasks attempted
day-135 (2026-07-13 13:41:02): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-135 (2026-07-13 12:22:40): tasks 1/3 ⚠️ — 1/3 strict verified; task states: obsolete_already_satisfied=1, reverted_unlanded_source_edits=1
day-135 (2026-07-13 04:55:55): tasks 0/2 ⚠️ — 0/2 strict verified; task states: reverted_unlanded_source_edits=1, reverted_unverified=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-136-20260714T095920Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 1.0
- primary fitness: task_success_rate=1.0, task_verification_rate=1.0
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Close yyds state and model lifecycle gaps (state_run_unmatched_non_validation_completed_count=22): Lifecycle causes: state_unmatched/open_after_FailureObserved=7; state...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=5): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile state-only tool failures (state_only_failed_tool_count=52): State events contained failed tool actions without matching transcrip...
- Recover failed tool actions before scoring (tool_error_count=1): Failed tool actions were present in session evidence; inspect the dom...
- Ignore prose-only DeepSeek cache ratios (deepseek_cache_ratio_unverified_count=1): DeepSeek cache ratios were reported without token-backed cache metric...

## GitHub Actions log feedback
latest score=0.8125 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- failed tool actions were recovered from transcripts -> inspect failed tool calls and add prompt/tool guards for the dominant failure class
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 5x │ command timed out after 120s
- 4x │ command timed out after 15s
... (truncated to fit token budget)
