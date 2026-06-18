# YOUR TRAJECTORY

Last computed: 2026-06-18T11:55Z. Day 110. Window: last 10 sessions / 14 days.
_Snapshot age: 424m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-110 (2026-06-18 04:51:02): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
day-109 (2026-06-17 23:44:22): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
day-109 (2026-06-17 20:46:55): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-109 (2026-06-17 18:46:07): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-109 (2026-06-17 17:25:01): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-109 (2026-06-17 12:37:13): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_no_edit=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-110-20260618T040525Z: classification=verified_success, can_drive_evolution=true
- warning: implementation terminal marker missing on 1 attempt(s); mechanical task proof exists
- evidence: provider_error_count=0, selected_task_count=3, tasks_attempted=3, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=2): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=13): State events contained failed tool actions without matching transcrip...
- Emit terminal markers after verified commits (task_terminal_marker_missing_attempt_count=1): Implementation landed mechanical proof but omitted the exact TASK_TER...
- Reduce successful-task turn overhead (max_task_turn_count=29): A verified task still used many turns, suggesting discovery or verifi...
- Ignore prose-only DeepSeek cache ratios (deepseek_cache_ratio_unverified_count=1): DeepSeek cache ratios were reported without token-backed cache metric...

## Structured state snapshot
claims: 547/666 proven; 119 non-proven (missing=89, observed=30); 2 recent; recent non-proven claims: assessment_artifact=1 observed, run_lifecycle=1 missing
- lifecycle aggregate: observed=65/74, unhealthy=38, run_incomplete=109, model_incomplete=53
- recent task issues: reverted_no_edit=1
- recent task expected evidence: task_01=Future `state summary` can cite reconciled counts Dashboard/trajectory can distinguish "or
- recent assessment artifacts: missing_with_diagnostic=1
- recent tool failures: unrecovered=6/15, failed_commands=12
- recent action evidence: state_only_failed_tools=13, transcript_only_failed_tools=2
- gnome evidence audit: adjusted=2787 across 74 session(s), top_sources=log_feedback=2076, task_artifacts=202, state_lifecycle.runs=192; reconciliation_not_raw_bug_count
- historical unrecovered tool failures: search_regex_error=57 addressed, bash_tool_error=51, tool_error=24
... (truncated to fit token budget)
