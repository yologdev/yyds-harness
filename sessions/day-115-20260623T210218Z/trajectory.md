# YOUR TRAJECTORY

Last computed: 2026-06-23T21:06Z. Day 115. Window: last 10 sessions / 14 days.
_Snapshot age: 140m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-115 (2026-06-23 18:45:53): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-115 (2026-06-23 18:07:10): tasks 1/1 ⚠️ — 0/1 strict verified; raw outcome 1/1; 1 no touched files; 1 no passing verifier
day-115 (2026-06-23 11:36:19): tasks 1/1 ⚠️ — 0/1 strict verified; raw outcome 1/1; 1 no touched files; 1 no passing verifier
day-115 (2026-06-23 04:01:35): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_no_edit=1
day-114 (2026-06-22 23:43:05): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-114 (2026-06-22 19:51:52): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-115-20260623T180824Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Close yyds state and model lifecycle gaps (state_run_incomplete_count=1): Lifecycle causes: state_incomplete/open_after_RunStarted=1; gaps: run...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=4): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=3): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=32): State events contained failed tool actions without matching transcrip...
- Recover failed tool actions before scoring (tool_error_count=1): Failed tool actions were present in session evidence; inspect the dom...

## GitHub Actions log feedback
latest score=0.9531 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- failed tool actions were recovered from transcripts -> inspect failed tool calls and add prompt/tool guards for the dominant failure class
- state run lifecycle was incomplete: state_incomplete/open_after_RunStarted=1 -> emit RunCompleted events for every started run, including timeout and API-error exits

## Structured state snapshot
claims: 762/891 proven; 129 non-proven (missing=97, observed=32); 1 recent; recent non-proven claims: run_lifecycle=1 missing
- lifecycle gaps: state_incomplete=1
- lifecycle causes: state_incomplete/open_after_RunStarted=1
- lifecycle aggregate: observed=90/99, unhealthy=46, run_incomplete=118, model_incomplete=54
- recent task issues: reverted_no_edit=2
... (truncated to fit token budget)
