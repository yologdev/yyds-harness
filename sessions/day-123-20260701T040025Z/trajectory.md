# YOUR TRAJECTORY

Last computed: 2026-07-01T04:04Z. Day 123. Window: last 10 sessions / 14 days.
_Snapshot age: 589m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-122 (2026-06-30 18:14:53): tasks 0/0 • — no tasks attempted
day-122 (2026-06-30 11:50:23): tasks 1/3 ⚠️ — 1/3 strict verified; task states: reverted_unlanded_source_edits=2
day-122 (2026-06-30 04:45:31): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unlanded_source_edits=1
day-121 (2026-06-29 18:36:45): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-121 (2026-06-29 13:06:01): tasks 0/0 • — no tasks attempted
day-121 (2026-06-29 04:42:11): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-122-20260630T175543Z: classification=no_task_evidence, can_drive_evolution=false
- issue: no selected or attempted task evidence captured; task success is not measurable
- evidence: provider_error_count=0, selected_task_count=0, tasks_attempted=0, task_artifact_coverage=0.0, task_lineage_capture_coverage=1.0
- action: repair planning/task selection so the next run captures selected tasks, attempted tasks, and verifier evidence before scoring evolution

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: unknown
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Make planning failure actionable (planner_no_task_count=1): The planner produced no concrete task files.
- Close yyds state and model lifecycle gaps (state_run_incomplete_count=1): Lifecycle causes: state_incomplete/open_after_SessionStarted=1; gaps:...
- Raise session success rate (session_success_rate=0.0): The evo session did not complete cleanly even though task success was...
- Validate seeded tasks against fresh assessment (task_seed_contradiction_count=1): Seeded tasks were contradicted by assessment evidence; validate seeds...
- Bound evaluator checks so verdicts are not skipped (evaluator_unverified_count=1): Recent task session day-122-20260630T105758Z: Some task evals were un...

## GitHub Actions log feedback
latest score=0.6781 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- seeded tasks contradicted the fresh assessment -> validate seeded tasks against fresh assessment evidence and replace contradicted seeds before implementation
- state run lifecycle was incomplete: state_incomplete/open_after_SessionStarted=1 -> emit RunCompleted events for every started run, including timeout and API-error exits
... (truncated to fit token budget)
