# YOUR TRAJECTORY

Last computed: 2026-06-22T04:25Z. Day 114. Window: last 10 sessions / 14 days.
_Snapshot age: 290m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-113 (2026-06-21 23:35:29): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unlanded_source_edits=1
day-113 (2026-06-21 18:26:33): tasks 1/3 ⚠️ — 1/3 strict verified; task states: reverted_no_edit=2
day-113 (2026-06-21 11:37:12): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-113 (2026-06-21 04:37:16): tasks 0/1 ⚠️ — 0/1 strict verified; task states: obsolete_already_satisfied=1
day-112 (2026-06-20 18:04:15): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
day-112 (2026-06-20 10:59:13): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-113-20260621T230011Z: classification=actionable, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=2, tasks_attempted=2, task_success_rate=0.5, task_verification_rate=0.5, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Raise verified task success rate (task_success_rate=0.5): Dominant task failure: task_unlanded_source_count=1 (source edits not...
- Bound evaluator checks so verdicts are not skipped (evaluator_unverified_count=1): Some task evals were unverified or timed out.
- Make source-edit outcomes land or explain reverts (task_unlanded_source_count=1): A task touched source files without a landed source commit.
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=7): prefer bounded commands with explicit paths and inspect exit output b...
- Make evaluator timeouts resumable or cheaper (evaluator_timeout_count=1): Evaluator timeout friction still appears in action logs.

## GitHub Actions log feedback
latest score=0.6625 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.5 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- tasks lacked strict verifier evidence -> require bounded verifier evidence before counting task success
- task source edits were not landed in source commits -> verify task source edits are committed before marking task completion
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 2x │ command timed out after 120s

## Structured state snapshot
claims: 667/792 proven; 125 non-proven (missing=94, observed=31); 3 recent; recent non-proven claims: model_lifecycle=1 missing, model_lifecycle=1 observed, run_lifecycle=1 missing
- lifecycle aggregate: observed=79/88, unhealthy=43, run_incomplete=114, model_incomplete=54
... (truncated to fit token budget)
