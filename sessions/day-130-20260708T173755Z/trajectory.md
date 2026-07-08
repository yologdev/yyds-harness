# YOUR TRAJECTORY

Last computed: 2026-07-08T17:41Z. Day 130. Window: last 10 sessions / 14 days.
_Snapshot age: 377m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-130 (2026-07-08 11:24:18): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-130 (2026-07-08 05:08:51): tasks 1/1 ⚠️ — 1/3 strict verified; task states: not_attempted=2
day-130 (2026-07-08 05:04:28): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_unlanded_source_edits=1
day-129 (2026-07-07 20:02:51): tasks 0/0 • — no tasks attempted
day-129 (2026-07-07 19:22:57): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-129 (2026-07-07 13:07:04): tasks 0/1 ⚠️ — 0/2 strict verified; task states: reverted_unverified=1, scope_mismatch=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-130-20260708T102007Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 1.0
- primary fitness: task_success_rate=1.0, task_verification_rate=1.0
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Close yyds state and model lifecycle gaps (state_run_unmatched_non_validation_completed_count=2): Lifecycle causes: state_unmatched/open_after_FailureObserved=2; gaps:...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=11): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile state-only tool failures (state_only_failed_tool_count=32): State events contained failed tool actions without matching transcrip...
- Recover failed tool actions before scoring (tool_error_count=1): Failed tool actions were present in session evidence; inspect the dom...
- Prefer bounded diagnostics before broad commands (command_timeout_count=1): Command timeouts slowed the coding loop.

## GitHub Actions log feedback
latest score=0.8906 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- commands timed out during the session -> prefer bounded targeted checks and record timeout-specific remediation
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 4x error: test failed, to rerun pass `--lib`
... (truncated to fit token budget)
