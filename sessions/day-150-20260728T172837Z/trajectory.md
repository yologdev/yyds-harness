# YOUR TRAJECTORY

Last computed: 2026-07-28T17:32Z. Day 150. Window: last 10 sessions / 14 days.
_Snapshot age: 342m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-150 (2026-07-28 11:49:58): tasks 1/3 ⚠️ — 1/3 strict verified; task states: obsolete_already_satisfied=2
day-150 (2026-07-28 03:26:19): tasks 0/0 • — no tasks attempted
day-149 (2026-07-27 18:24:15): tasks 0/0 • — no tasks attempted
day-149 (2026-07-27 12:15:01): tasks 0/0 • — no tasks attempted
day-149 (2026-07-27 03:54:56): tasks 0/0 • — no tasks attempted
day-148 (2026-07-26 18:12:22): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-150-20260728T103600Z: classification=actionable, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=3, tasks_attempted=3, task_success_rate=0.3333333333333333, task_verification_rate=0.3333333333333333, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 0.3333
- primary fitness: task_success_rate=0.3333333333333333, task_verification_rate=0.3333333333333333
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Raise verified task success rate (task_success_rate=0.3333333333333333): Dominant task failure: task_obsolete_count=2 (obsolete selected tasks...
- Make source-edit outcomes land or explain reverts (task_unlanded_source_count=1): A task touched source files without a landed source commit.
- Require strict verifier evidence for tasks (task_verification_rate=0.3333333333333333): Task verification rate was below complete without a counted evaluator...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=3): prefer bounded commands with explicit paths and inspect exit output b...
- Replace stale or already-satisfied tasks (task_obsolete_count=2): Implementation marked selected tasks obsolete or already satisfied; p...

## GitHub Actions log feedback
latest score=0.5458 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.3333333333333333 task_spec_quality_score=0.8
Corrected top lessons for next run:
- failed tool actions were recovered from transcripts -> inspect failed tool calls and add prompt/tool guards for the dominant failure class
- task source edits were not landed in source commits -> verify task source edits are committed before marking task completion
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 2x │ command timed out after 240s. add an explicit timeout parameter (e.g. timeout: 600) for 

... (truncated to fit token budget)
