# YOUR TRAJECTORY

Last computed: 2026-06-24T19:19Z. Day 116. Window: last 10 sessions / 14 days.
_Snapshot age: 64m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-116 (2026-06-24 18:14:58): tasks 0/0 • — no tasks attempted
day-116 (2026-06-24 11:17:09): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-116 (2026-06-24 03:55:29): tasks 0/0 • — no tasks attempted
day-116 (2026-06-24 01:01:36): tasks 1/3 ⚠️ — 1/3 strict verified; task states: reverted_no_edit=1, reverted_unlanded_source_edits=1
day-115 (2026-06-23 21:40:56): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_no_edit=1
day-115 (2026-06-23 18:45:53): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-116-20260624T175532Z: classification=no_task_evidence, can_drive_evolution=false
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
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=2): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=2): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=36): State events contained failed tool actions without matching transcrip...

## GitHub Actions log feedback
latest score=0.725 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0
Corrected top lessons for next run:
- planner produced no usable task -> bound discovery and require a selected task artifact before implementation work starts

## Structured state snapshot
claims: 801/936 proven; 135 non-proven (missing=101, observed=34); 6 recent; recent non-proven claims: run_lifecycle=4 missing, assessment_artifact=1 observed, model_lifecycle=1 observed
- lifecycle aggregate: observed=95/104, unhealthy=50, run_incomplete=121, model_incomplete=54
- recent task issues: reverted_no_edit=2, reverted_unlanded_source_edits=1
... (truncated to fit token budget)
