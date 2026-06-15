# YOUR TRAJECTORY

Last computed: 2026-06-15T20:20Z. Day 107. Window: last 10 sessions / 14 days.
_Snapshot age: 172m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-107 (2026-06-15 17:28:17): tasks 1/2 ⚠️ — 0/2 strict verified; raw outcome 1/2; task states: reverted_seed_contradicted=1, scope_mismatch=1
day-107 (2026-06-15 15:08:03): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
day-107 (2026-06-15 13:04:31): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
day-107 (2026-06-15 11:17:05): tasks 0/3 ⚠️ — 0/3 strict verified; task states: reverted_unlanded_source_edits=2, reverted_seed_contradicted=1
day-107 (2026-06-15 09:58:03): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
day-107 (2026-06-15 04:42:50): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-107-20260615T165013Z: classification=actionable, can_drive_evolution=true
- warning: task implementation terminal evidence incomplete for 1 task artifact(s)
- evidence: provider_error_count=0, selected_task_count=2, tasks_attempted=2, task_success_rate=0.5, task_verification_rate=0.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Raise verified task success rate (task_success_rate=0.5): Dominant task failure: task_analysis_only_attempt_count=2 (analysis-o...
- Force analysis-only attempts into action (task_analysis_only_attempt_count=2): Implementation ended without file progress or terminal evidence; retr...
- Validate seeded tasks against fresh assessment (task_seed_contradiction_count=1): Seeded tasks were contradicted by assessment evidence; validate seeds...
- Require strict verifier evidence for tasks (task_verification_rate=0.0): Task verification rate was below complete without a counted evaluator...
- Require terminal task evidence before completion (task_incomplete_terminal_count=1): Implementation exited cleanly without TASK_TERMINAL_EVIDENCE or mecha...

## GitHub Actions log feedback
latest score=0.7328 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.5 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- edit failed because the replacement context was ambiguous or absent -> read a tighter surrounding range and use unique old_text context before applying edits
- seeded tasks contradicted the fresh assessment -> validate seeded tasks against fresh assessment evidence and replace contradicted seeds before implementation
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 4x error: test failed, to rerun pass `--lib`
... (truncated to fit token budget)
