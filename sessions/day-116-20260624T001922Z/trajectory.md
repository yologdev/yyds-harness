# YOUR TRAJECTORY

Last computed: 2026-06-24T00:23Z. Day 116. Window: last 10 sessions / 14 days.
_Snapshot age: 162m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-115 (2026-06-23 21:40:56): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_no_edit=1
day-115 (2026-06-23 18:45:53): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-115 (2026-06-23 18:07:10): tasks 1/1 ⚠️ — 0/1 strict verified; raw outcome 1/1; 1 no touched files; 1 no passing verifier
day-115 (2026-06-23 11:36:19): tasks 1/1 ⚠️ — 0/1 strict verified; raw outcome 1/1; 1 no touched files; 1 no passing verifier
day-115 (2026-06-23 04:01:35): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_no_edit=1
day-114 (2026-06-22 23:43:05): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-115-20260623T210218Z: classification=actionable, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=3, tasks_attempted=3, task_success_rate=0.6666666666666666, task_verification_rate=0.6666666666666666, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 0.5344
- primary fitness: task_success_rate=0.6666666666666666, task_verification_rate=0.6666666666666666, coding_log_score=0.8042, session_success_rate=0.0
- diagnostic gates: planner_no_task_count=0, provider_error_count=0, evaluator_timeout_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Force analysis-only attempts into action (task_analysis_only_attempt_count=1): Implementation ended without file progress or terminal evidence; retr...
- Raise verified task success rate (task_success_rate=0.6666666666666666): Dominant task failure: task_analysis_only_attempt_count=1 (analysis-o...
- Require strict verifier evidence for tasks (task_verification_rate=0.6666666666666666): Task verification rate was below complete without a counted evaluator...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=4): prefer bounded commands with explicit paths and inspect exit output b...
- Close yyds state and model lifecycle gaps (state_run_incomplete_count=1): Lifecycle causes: state_incomplete/open_after_SessionStarted=1; gaps:...

## GitHub Actions log feedback
latest score=0.8042 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.6666666666666666 task_spec_quality_score=1.0
Corrected top lessons for next run:
... (truncated to fit token budget)
