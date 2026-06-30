# YOUR TRAJECTORY

Last computed: 2026-06-30T03:47Z. Day 122. Window: last 10 sessions / 14 days.
_Snapshot age: 550m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-121 (2026-06-29 18:36:45): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-121 (2026-06-29 13:06:01): tasks 0/0 • — no tasks attempted
day-121 (2026-06-29 04:42:11): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-120 (2026-06-28 17:32:32): tasks 0/0 • — no tasks attempted
day-120 (2026-06-28 10:46:04): tasks 0/0 • — no tasks attempted
day-120 (2026-06-28 04:40:57): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unlanded_source_edits=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-121-20260629T180925Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Capability fitness feedback
- goal: improve yyds DeepSeek coding/general-agent capability
- fitness_score: 1.0
- primary fitness: task_success_rate=1.0, task_verification_rate=1.0
- diagnostic gates: provider_error_count=0
- action: choose tasks that raise fitness gnomes or add held-out coding eval evidence; treat diagnostics as gates, not the final goal

## Graph-derived next-task pressure
- Break recurring log failure fingerprints (recurring_failure_count=1): GitHub/action log feedback repeated failure fingerprints across sessi...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=6): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=5): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=55): State events contained failed tool actions without matching transcrip...
- Recover failed tool actions before scoring (tool_error_count=4): Failed tool actions were present in session evidence; inspect the dom...

## GitHub Actions log feedback
latest score=0.8281 confidence=1.0 recurring_failures=1 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
- agent read or searched paths that did not exist -> verify guessed paths with rg --files before reading them, then search owning symbols instead of retrying absent paths
- commands timed out during the session -> prefer bounded targeted checks and record timeout-specific remediation
... (truncated to fit token budget)
