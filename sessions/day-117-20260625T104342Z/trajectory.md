# YOUR TRAJECTORY

Last computed: 2026-06-25T10:47Z. Day 117. Window: last 10 sessions / 14 days.
_Snapshot age: 410m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-117 (2026-06-25 03:57:40): tasks 0/0 • — no tasks attempted
day-117 (2026-06-25 01:12:53): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_unlanded_source_edits=1
day-116 (2026-06-24 19:38:28): tasks 0/2 ⚠️ — 0/2 strict verified; task states: reverted_no_edit=2
day-116 (2026-06-24 18:14:58): tasks 0/0 • — no tasks attempted
day-116 (2026-06-24 11:17:09): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-116 (2026-06-24 03:55:29): tasks 0/0 • — no tasks attempted
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-117-20260625T033942Z: classification=no_task_evidence, can_drive_evolution=false
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
- Force analysis-only attempts into action (task_analysis_only_attempt_count=1): Recent task session day-117-20260625T003550Z: Implementation ended wi...
- Make source-edit outcomes land or explain reverts (task_unlanded_source_count=1): Recent task session day-117-20260625T003550Z: A task touched source f...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=2): prefer bounded commands with explicit paths and inspect exit output b...

## GitHub Actions log feedback
latest score=0.7875 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0
Corrected top lessons for next run:
- planner produced no usable task -> bound discovery and require a selected task artifact before implementation work starts

## Structured state snapshot
claims: 827/963 proven; 136 non-proven (missing=102, observed=34); 2 recent; recent non-proven claims: run_lifecycle=2 missing
- lifecycle aggregate: observed=98/107, unhealthy=51, run_incomplete=122, model_incomplete=54
- recent task issues: reverted_no_edit=2, reverted_unlanded_source_edits=1
... (truncated to fit token budget)
