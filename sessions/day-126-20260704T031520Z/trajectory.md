# YOUR TRAJECTORY

Last computed: 2026-07-04T03:19Z. Day 126. Window: last 10 sessions / 14 days.
_Snapshot age: 553m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-125 (2026-07-03 18:05:20): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-125 (2026-07-03 11:20:51): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-125 (2026-07-03 03:43:42): tasks 0/0 • — no tasks attempted
day-124 (2026-07-02 18:29:34): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unlanded_source_edits=1
day-124 (2026-07-02 11:50:30): tasks 0/2 ⚠️ — 0/2 strict verified; task states: reverted_unlanded_source_edits=2
day-124 (2026-07-02 04:28:34): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_unlanded_source_edits=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-125-20260703T172714Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=2, tasks_attempted=2, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 1.0
- primary fitness: task_success_rate=1.0, task_verification_rate=1.0
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=15): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=2): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=55): State events contained failed tool actions without matching transcrip...
- Recover failed tool actions before scoring (tool_error_count=2): Failed tool actions were present in session evidence; inspect the dom...

## GitHub Actions log feedback
latest score=0.9531 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- edit failed because the replacement context was ambiguous or absent -> read a tighter surrounding range and use unique old_text context before applying edits
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 3x │ command timed out after 30s
- 2x evaluator: timed out — failing task because no verifier verdict exists
... (truncated to fit token budget)
