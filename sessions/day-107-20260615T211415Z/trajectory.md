# YOUR TRAJECTORY

Last computed: 2026-06-15T21:17Z. Day 107. Window: last 10 sessions / 14 days.
_Snapshot age: 4m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-107 (2026-06-15 21:13:04): tasks 0/2 ⚠️ — 0/2 strict verified; task states: reverted_unlanded_source_edits=2
day-107 (2026-06-15 17:28:17): tasks 1/2 ⚠️ — 0/2 strict verified; raw outcome 1/2; task states: reverted_seed_contradicted=1
day-107 (2026-06-15 15:08:03): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
day-107 (2026-06-15 13:04:31): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
day-107 (2026-06-15 11:17:05): tasks 0/3 ⚠️ — 0/3 strict verified; task states: reverted_unlanded_source_edits=2, reverted_seed_contradicted=1
day-107 (2026-06-15 09:58:03): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-107-20260615T201750Z: classification=actionable, can_drive_evolution=true
- warning: task implementation terminal evidence incomplete for 2 task artifact(s)
- evidence: provider_error_count=0, selected_task_count=2, tasks_attempted=2, task_success_rate=0.0, task_verification_rate=0.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Close yyds state and model lifecycle gaps (state_run_incomplete_count=1): Lifecycle causes: state_incomplete/open_after_SessionStarted=1; gaps:...
- Raise verified task success rate (task_success_rate=0.0): Dominant task failure: task_unlanded_source_count=2 (source edits not...
- Validate seeded tasks against fresh assessment (task_seed_contradiction_count=1): Seeded tasks were contradicted by assessment evidence; validate seeds...
- Make source-edit outcomes land or explain reverts (task_unlanded_source_count=2): A task touched source files without a landed source commit.
- Require strict verifier evidence for tasks (task_verification_rate=0.0): Task verification rate was below complete without a counted evaluator...

## GitHub Actions log feedback
latest score=0.5825 confidence=1.0 recurring_failures=3 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- edit failed because the replacement context was ambiguous or absent -> read a tighter surrounding range and use unique old_text context before applying edits
- tasks lacked strict verifier evidence -> require bounded verifier evidence before counting task success
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 5x error: test failed, to rerun pass `--lib`
- 4x thread 'state::tests::run_completion_guard_reports_error_on_panic' (<n>) panicked at src/s
... (truncated to fit token budget)
