# YOUR TRAJECTORY

Last computed: 2026-07-08T10:24Z. Day 130. Window: last 10 sessions / 14 days.
_Snapshot age: 315m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-130 (2026-07-08 05:08:51): tasks 1/1 ⚠️ — 1/3 strict verified; task states: not_attempted=2
day-130 (2026-07-08 05:04:28): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_unlanded_source_edits=1
day-129 (2026-07-07 20:02:51): tasks 0/0 • — no tasks attempted
day-129 (2026-07-07 19:22:57): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-129 (2026-07-07 13:07:04): tasks 0/1 ⚠️ — 0/2 strict verified; task states: reverted_unverified=1, scope_mismatch=1
day-129 (2026-07-07 05:57:38): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-130-20260708T041112Z: classification=not_ready, can_drive_evolution=false
- issue: task lineage capture incomplete: 0.3333333333333333
- evidence: provider_error_count=0, selected_task_count=3, tasks_attempted=1, task_success_rate=1.0, task_verification_rate=0.3333333333333333, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: repair the named evidence gap before trusting the next evolution step

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 0.3333
- primary fitness: task_success_rate=0.3333333333333333, task_verification_rate=0.3333333333333333
- diagnostic gates: provider_error_count=0
- blocker: diagnostic gate(s) still obscure capability fitness: task_lineage_capture_coverage

## Graph-derived next-task pressure
- Close yyds state and model lifecycle gaps (deepseek_model_call_unmatched_completed_count=16): Lifecycle causes: state_unmatched/completion_without_run_start=8; gap...
- Preserve budget to start every selected task (task_unattempted_count=2): The planner selected tasks that the implementation phase never attemp...
- Require strict verifier evidence for tasks (task_verification_rate=0.3333333333333333): Task verification rate was below complete without a counted evaluator...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=10): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=1): Recent transcripts contained failed tool actions absent from state ev...

## GitHub Actions log feedback
latest score=0.8125 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 5x error: test failed, to rerun pass `--lib`
... (truncated to fit token budget)
