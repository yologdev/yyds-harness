# YOUR TRAJECTORY

Last computed: 2026-06-17T04:17Z. Day 109. Window: last 10 sessions / 14 days.
_Snapshot age: 393m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-108 (2026-06-16 21:44:09): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-108 (2026-06-16 18:04:37): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-108 (2026-06-16 17:17:37): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unverified=1
day-108 (2026-06-16 15:25:46): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-108 (2026-06-16 14:30:29): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_unlanded_source_edits=1
day-108 (2026-06-16 13:28:27): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-108-20260616T212224Z: classification=actionable, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=2, tasks_attempted=2, task_success_rate=0.5, task_verification_rate=0.5, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Force analysis-only attempts into action (task_analysis_only_attempt_count=2): Implementation ended without file progress or terminal evidence; retr...
- Force reverted tasks to leave concrete evidence (task_no_edit_revert_count=1): Implementation tasks reverted without touching files; require an earl...
- Raise verified task success rate (task_success_rate=0.5): Dominant task failure: task_analysis_only_attempt_count=2 (analysis-o...
- Require strict verifier evidence for tasks (task_verification_rate=0.5): Task verification rate was below complete without a counted evaluator...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=5): prefer bounded commands with explicit paths and inspect exit output b...

## GitHub Actions log feedback
latest score=0.7719 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.5 task_spec_quality_score=1.0
Corrected top lessons for next run:
- edit failed because the replacement context was ambiguous or absent -> read a tighter surrounding range and use unique old_text context before applying edits
- implementation tasks reverted without edits -> force implementation agents to either make an early scoped edit, write an obsolete note, or fail with a concrete blocker
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 2x error: test failed, to rerun pass `--lib`
- 2x thread 'empty_piped_stdin_exits_quickly' (<n>) panicked at tests/integration.rs:n:n:
- 2x error: test failed, to rerun pass `--test integration`
... (truncated to fit token budget)
