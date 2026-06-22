# YOUR TRAJECTORY

Last computed: 2026-06-22T19:33Z. Day 114. Window: last 10 sessions / 14 days.
_Snapshot age: 220m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-114 (2026-06-22 15:53:07): tasks 1/2 ⚠️ — 1/2 strict verified; task states: obsolete_already_satisfied=1
day-114 (2026-06-22 14:02:07): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-114 (2026-06-22 13:01:24): tasks 0/1 ⚠️ — 0/1 strict verified; task states: obsolete_already_satisfied=1
day-114 (2026-06-22 09:28:16): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-114 (2026-06-22 04:49:15): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-113 (2026-06-21 23:35:29): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unlanded_source_edits=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-114-20260622T152419Z: classification=actionable, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=2, tasks_attempted=2, task_success_rate=0.5, task_verification_rate=0.5, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Raise verified task success rate (task_success_rate=0.5): Dominant task failure: task_obsolete_count=1 (obsolete selected tasks...
- Require strict verifier evidence for tasks (task_verification_rate=0.5): Task verification rate was below complete without a counted evaluator...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=4): prefer bounded commands with explicit paths and inspect exit output b...
- Replace stale or already-satisfied tasks (task_obsolete_count=1): Implementation marked selected tasks obsolete or already satisfied; p...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=1): Recent transcripts contained failed tool actions absent from state ev...

## GitHub Actions log feedback
latest score=0.7719 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=0.5 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- agent read or searched paths that did not exist -> verify guessed paths with rg --files before reading them, then search owning symbols instead of retrying absent paths

## Structured state snapshot
claims: 709/837 proven; 128 non-proven (missing=96, observed=32); 3 recent; recent non-proven claims: run_lifecycle=2 missing, model_lifecycle=1 observed
- lifecycle aggregate: observed=84/93, unhealthy=45, run_incomplete=117, model_incomplete=54
- recent tool failures: unrecovered=7/35, failed_commands=33
- recent action evidence: state_only_failed_tools=34, transcript_only_failed_tools=1
... (truncated to fit token budget)
