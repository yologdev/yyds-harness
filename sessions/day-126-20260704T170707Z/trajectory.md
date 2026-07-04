# YOUR TRAJECTORY

Last computed: 2026-07-04T17:10Z. Day 126. Window: last 10 sessions / 14 days.
_Snapshot age: 361m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-126 (2026-07-04 11:08:11): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-126 (2026-07-04 03:47:33): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_unlanded_source_edits=1
day-125 (2026-07-03 18:05:20): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-125 (2026-07-03 11:20:51): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-125 (2026-07-03 03:43:42): tasks 0/0 • — no tasks attempted
day-124 (2026-07-02 18:29:34): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unlanded_source_edits=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-126-20260704T101107Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=2, tasks_attempted=2, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 1.0
- primary fitness: task_success_rate=1.0, task_verification_rate=1.0
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Close yyds state and model lifecycle gaps (state_run_incomplete_count=1): Lifecycle causes: state_incomplete/open_after_SessionStarted=1; gaps:...
- Break recurring log failure fingerprints (recurring_failure_count=2): GitHub/action log feedback repeated failure fingerprints across sessi...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=10): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=4): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=53): State events contained failed tool actions without matching transcrip...

## GitHub Actions log feedback
latest score=0.8125 confidence=1.0 recurring_failures=2 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- file-read evidence contained path or access errors -> verify paths with rg --files and prefer module or symbol discovery when exact files are uncertain
... (truncated to fit token budget)
