# YOUR TRAJECTORY

Last computed: 2026-06-28T17:17Z. Day 120. Window: last 10 sessions / 14 days.
_Snapshot age: 391m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-120 (2026-06-28 10:46:04): tasks 0/0 • — no tasks attempted
day-120 (2026-06-28 04:40:57): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unlanded_source_edits=1
day-119 (2026-06-27 17:35:02): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_no_edit=1
day-119 (2026-06-27 10:30:01): tasks 0/0 • — no tasks attempted
day-119 (2026-06-27 03:50:53): tasks 0/0 • — no tasks attempted
day-118 (2026-06-26 22:26:23): tasks 0/0 • — no tasks attempted
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-120-20260628T102900Z: classification=no_task_evidence, can_drive_evolution=false
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
- Raise session success rate (session_success_rate=0.0): The evo session did not complete cleanly even though task success was...
- Bound evaluator checks so verdicts are not skipped (evaluator_unverified_count=1): Recent task session day-120-20260628T035643Z: Some task evals were un...
- Make source-edit outcomes land or explain reverts (task_unlanded_source_count=1): Recent task session day-120-20260628T035643Z: A task touched source f...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=3): prefer bounded commands with explicit paths and inspect exit output b...

## GitHub Actions log feedback
latest score=0.8031 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0
Corrected top lessons for next run:
- planner produced no usable task -> bound discovery and require a selected task artifact before implementation work starts

## Structured state snapshot
claims: 930/1071 proven; 141 non-proven (missing=104, observed=37); 4 recent; recent non-proven claims: model_lifecycle=2 observed, run_lifecycle=2 missing
- lifecycle aggregate: observed=110/119, unhealthy=55, run_incomplete=124, model_incomplete=54
- recent task issues: reverted_no_edit=1, reverted_unlanded_source_edits=1
... (truncated to fit token budget)
