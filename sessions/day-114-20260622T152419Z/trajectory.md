# YOUR TRAJECTORY

Last computed: 2026-06-22T15:28Z. Day 114. Window: last 10 sessions / 14 days.
_Snapshot age: 86m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-114 (2026-06-22 14:02:07): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-114 (2026-06-22 13:01:24): tasks 0/1 ⚠️ — 0/1 strict verified; task states: obsolete_already_satisfied=1
day-114 (2026-06-22 09:28:16): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-114 (2026-06-22 04:49:15): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-113 (2026-06-21 23:35:29): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unlanded_source_edits=1
day-113 (2026-06-21 18:26:33): tasks 1/3 ⚠️ — 1/3 strict verified; task states: reverted_no_edit=2
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-114-20260622T133602Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=6): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile state-only tool failures (state_only_failed_tool_count=39): State events contained failed tool actions without matching transcrip...
- Recover failed tool actions before scoring (tool_error_count=1): Failed tool actions were present in session evidence; inspect the dom...
- Ignore prose-only DeepSeek cache ratios (deepseek_cache_ratio_unverified_count=2): DeepSeek cache ratios were reported without token-backed cache metric...

## GitHub Actions log feedback
latest score=0.9844 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- failed tool actions were recovered from transcripts -> inspect failed tool calls and add prompt/tool guards for the dominant failure class

## Structured state snapshot
claims: 700/828 proven; 128 non-proven (missing=96, observed=32); 4 recent; recent non-proven claims: model_lifecycle=2 observed, run_lifecycle=2 missing
- lifecycle aggregate: observed=83/92, unhealthy=45, run_incomplete=117, model_incomplete=54
- recent task issues: reverted_unlanded_source_edits=1
- recent task expected evidence: task_02=`yyds state failures tools` shows bash failures after a session where bash commands exited
- recent tool failures: unrecovered=8/39, failed_commands=38
- recent action evidence: state_only_failed_tools=39
- gnome evidence audit: adjusted=3577 across 92 session(s), top_sources=log_feedback=2847, task_artifacts=203, state_lifecycle.runs=192; reconciliation_not_raw_bug_count
... (truncated to fit token budget)
