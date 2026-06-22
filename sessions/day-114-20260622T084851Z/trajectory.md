# YOUR TRAJECTORY

Last computed: 2026-06-22T08:52Z. Day 114. Window: last 10 sessions / 14 days.
_Snapshot age: 243m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-114 (2026-06-22 04:49:15): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-113 (2026-06-21 23:35:29): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unlanded_source_edits=1
day-113 (2026-06-21 18:26:33): tasks 1/3 ⚠️ — 1/3 strict verified; task states: reverted_no_edit=2
day-113 (2026-06-21 11:37:12): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-113 (2026-06-21 04:37:16): tasks 0/1 ⚠️ — 0/1 strict verified; task states: obsolete_already_satisfied=1
day-112 (2026-06-20 18:04:15): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-114-20260622T042144Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=2, tasks_attempted=2, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Close yyds state and model lifecycle gaps (state_run_incomplete_count=2): Lifecycle causes: state_incomplete/open_after_SessionStarted=2; gaps:...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=7): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=1): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=32): State events contained failed tool actions without matching transcrip...
- Recover failed tool actions before scoring (tool_error_count=6): Failed tool actions were present in session evidence; inspect the dom...

## GitHub Actions log feedback
latest score=0.8438 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- failed tool actions were recovered from transcripts -> inspect failed tool calls and add prompt/tool guards for the dominant failure class
- state run lifecycle was incomplete: state_incomplete/open_after_SessionStarted=2 -> emit RunCompleted events for every started run, including timeout and API-error exits

## Structured state snapshot
claims: 675/801 proven; 126 non-proven (missing=95, observed=31); 2 recent; recent non-proven claims: model_lifecycle=1 observed, run_lifecycle=1 missing
- lifecycle gaps: state_incomplete=2
- lifecycle causes: state_incomplete/open_after_SessionStarted=2
- lifecycle aggregate: observed=80/89, unhealthy=44, run_incomplete=116, model_incomplete=54
- recent task issues: reverted_no_edit=2, reverted_unlanded_source_edits=1
... (truncated to fit token budget)
