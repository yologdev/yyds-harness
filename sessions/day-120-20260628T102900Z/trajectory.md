# YOUR TRAJECTORY

Last computed: 2026-06-28T10:32Z. Day 120. Window: last 10 sessions / 14 days.
_Snapshot age: 351m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-120 (2026-06-28 04:40:57): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unlanded_source_edits=1
day-119 (2026-06-27 17:35:02): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_no_edit=1
day-119 (2026-06-27 10:30:01): tasks 0/0 • — no tasks attempted
day-119 (2026-06-27 03:50:53): tasks 0/0 • — no tasks attempted
day-118 (2026-06-26 22:26:23): tasks 0/0 • — no tasks attempted
day-118 (2026-06-26 21:28:05): tasks 0/0 • — no tasks attempted
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-120-20260628T035643Z: classification=actionable, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=2, tasks_attempted=2, task_success_rate=0.5, task_verification_rate=0.5, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 0.5
- primary fitness: task_success_rate=0.5, task_verification_rate=0.5
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Raise verified task success rate (task_success_rate=0.5): Dominant task failure: task_unlanded_source_count=1 (source edits not...
- Bound evaluator checks so verdicts are not skipped (evaluator_unverified_count=1): Some task evals were unverified or timed out.
- Make source-edit outcomes land or explain reverts (task_unlanded_source_count=1): A task touched source files without a landed source commit.
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=3): prefer bounded commands with explicit paths and inspect exit output b...
- Make evaluator timeouts resumable or cheaper (evaluator_timeout_count=1): Evaluator timeout friction still appears in action logs.

## GitHub Actions log feedback
latest score=0.6625 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.5 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- edit failed because the replacement context was ambiguous or absent -> read a tighter surrounding range and use unique old_text context before applying edits
- tasks lacked strict verifier evidence -> require bounded verifier evidence before counting task success
... (truncated to fit token budget)
