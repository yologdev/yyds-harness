# YOUR TRAJECTORY

Last computed: 2026-06-21T11:21Z. Day 113. Window: last 10 sessions / 14 days.
_Snapshot age: 403m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-113 (2026-06-21 04:37:16): tasks 0/1 ⚠️ — 0/1 strict verified; task states: obsolete_already_satisfied=1
day-112 (2026-06-20 18:04:15): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
day-112 (2026-06-20 10:59:13): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-112 (2026-06-20 04:08:32): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-111 (2026-06-19 18:26:07): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-111 (2026-06-19 12:42:23): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-113-20260621T041931Z: classification=actionable, can_drive_evolution=true
- warning: task implementation terminal evidence incomplete for 1 task artifact(s)
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=0.0, task_verification_rate=0.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Raise verified task success rate (task_success_rate=0.0): Dominant task failure: task_obsolete_count=1 (obsolete selected tasks...
- Require strict verifier evidence for tasks (task_verification_rate=0.0): Task verification rate was below complete without a counted evaluator...
- Require terminal task evidence before completion (task_incomplete_terminal_count=1): Implementation exited cleanly without TASK_TERMINAL_EVIDENCE or mecha...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=6): prefer bounded commands with explicit paths and inspect exit output b...
- Replace stale or already-satisfied tasks (task_obsolete_count=1): Implementation marked selected tasks obsolete or already satisfied; p...

## GitHub Actions log feedback
latest score=0.7141 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 2x │ command timed out after 120s
- 2x error: test failed, to rerun pass `--lib`

## Structured state snapshot
claims: 641/765 proven; 124 non-proven (missing=94, observed=30); 2 recent; recent non-proven claims: model_lifecycle=1 missing, run_lifecycle=1 missing
- lifecycle aggregate: observed=76/85, unhealthy=42, run_incomplete=114, model_incomplete=54
- recent task issues: reverted_no_edit=1
... (truncated to fit token budget)
