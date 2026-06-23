# YOUR TRAJECTORY

Last computed: 2026-06-23T18:12Z. Day 115. Window: last 10 sessions / 14 days.
_Snapshot age: 5m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-115 (2026-06-23 18:07:10): tasks 1/1 ⚠️ — 0/1 strict verified; raw outcome 1/1; 1 no touched files; 1 no passing verifier
day-115 (2026-06-23 11:36:19): tasks 1/1 ⚠️ — 0/1 strict verified; raw outcome 1/1; 1 no touched files; 1 no passing verifier
day-115 (2026-06-23 04:01:35): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_no_edit=1
day-114 (2026-06-22 23:43:05): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-114 (2026-06-22 19:51:52): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-114 (2026-06-22 15:53:07): tasks 1/2 ⚠️ — 1/2 strict verified; task states: obsolete_already_satisfied=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-115-20260623T174924Z: classification=not_ready, can_drive_evolution=false
- issue: task artifact coverage incomplete: None
- evidence: provider_error_count=0, selected_task_count=0, tasks_attempted=1, task_success_rate=0.0, task_artifact_coverage=0.0, task_lineage_capture_coverage=1.0
- action: repair the named evidence gap before trusting the next evolution step

## Graph-derived next-task pressure
- Make planning failure actionable (planner_no_task_count=1): The planner produced no concrete task files.
- Restore task artifact coverage (task_artifact_coverage=0): Task decisions or artifacts were missing from the audit bundle.
- Raise verified task success rate (task_success_rate=0.0): Selected or attempted tasks did not all finish as verified successful...
- Validate seeded tasks against fresh assessment (task_seed_contradiction_count=1): Seeded tasks were contradicted by assessment evidence; validate seeds...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=4): prefer bounded commands with explicit paths and inspect exit output b...

## GitHub Actions log feedback
latest score=0.6281 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- seeded tasks contradicted the fresh assessment -> validate seeded tasks against fresh assessment evidence and replace contradicted seeds before implementation
- raw task success lacked strict task evidence -> show raw success as unverified until task artifacts and verifier rows prove it

## Structured state snapshot
claims: 754/882 proven; 128 non-proven (missing=96, observed=32); recent non-proven claims: run_lifecycle=52 missing, model_lifecycle=44 missing, assessment_artifact=25 observed
- lifecycle aggregate: observed=89/98, unhealthy=45, run_incomplete=117, model_incomplete=54
... (truncated to fit token budget)
