# YOUR TRAJECTORY

Last computed: 2026-06-23T11:22Z. Day 115. Window: last 10 sessions / 14 days.
_Snapshot age: 440m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-115 (2026-06-23 04:01:35): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_no_edit=1
day-114 (2026-06-22 23:43:05): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-114 (2026-06-22 19:51:52): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-114 (2026-06-22 15:53:07): tasks 1/2 ⚠️ — 1/2 strict verified; task states: obsolete_already_satisfied=1
day-114 (2026-06-22 14:02:07): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-114 (2026-06-22 13:01:24): tasks 0/1 ⚠️ — 0/1 strict verified; task states: obsolete_already_satisfied=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-115-20260623T033935Z: classification=actionable, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=0.0, task_verification_rate=0.0, task_artifact_coverage=0.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Make planning failure actionable (planner_no_task_count=1): The planner produced no concrete task files.
- Restore task artifact coverage (task_artifact_coverage=0): Task decisions or artifacts were missing from the audit bundle.
- Force analysis-only attempts into action (task_analysis_only_attempt_count=1): Implementation ended without file progress or terminal evidence; retr...
- Repair state replay integrity (state_replay_integrity_rate=0.0): State replay did not match recorded session artifacts; reconcile stat...
- Raise verified task success rate (task_success_rate=0.0): Dominant task failure: task_analysis_only_attempt_count=1 (analysis-o...

## GitHub Actions log feedback
latest score=0.7219 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- implementation tasks reverted without edits -> force implementation agents to either make an early scoped edit, write an obsolete note, or fail with a concrete blocker
- planner produced no usable task -> bound discovery and require a selected task artifact before implementation work starts

## Structured state snapshot
claims: 736/864 proven; 128 non-proven (missing=96, observed=32); recent non-proven claims: run_lifecycle=52 missing, model_lifecycle=44 missing, assessment_artifact=25 observed
- lifecycle aggregate: observed=87/96, unhealthy=45, run_incomplete=117, model_incomplete=54
- recent task issues: reverted_no_edit=3
... (truncated to fit token budget)
