# YOUR TRAJECTORY

Last computed: 2026-06-24T17:59Z. Day 116. Window: last 10 sessions / 14 days.
_Snapshot age: 402m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-116 (2026-06-24 11:17:09): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-116 (2026-06-24 03:55:29): tasks 0/0 • — no tasks attempted
day-116 (2026-06-24 01:01:36): tasks 1/3 ⚠️ — 1/3 strict verified; task states: reverted_no_edit=1, reverted_unlanded_source_edits=1
day-115 (2026-06-23 21:40:56): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_no_edit=1
day-115 (2026-06-23 18:45:53): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-115 (2026-06-23 18:07:10): tasks 1/1 ⚠️ — 0/1 strict verified; raw outcome 1/1; 1 no touched files; 1 no passing verifier
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-116-20260624T105137Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 1.0
- primary fitness: task_success_rate=1.0, task_verification_rate=1.0
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Close yyds state and model lifecycle gaps (state_run_incomplete_count=2): Lifecycle causes: state_incomplete/open_after_RunStarted=1; state_inc...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=3): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=32): State events contained failed tool actions without matching transcrip...
- Recover failed tool actions before scoring (tool_error_count=1): Failed tool actions were present in session evidence; inspect the dom...

## GitHub Actions log feedback
latest score=0.8438 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- state run lifecycle was incomplete: state_incomplete/open_after_RunStarted=1, state_incomplete/open_after_SessionStarted -> emit RunCompleted events for every started run, including timeout and API-error exits
... (truncated to fit token budget)
