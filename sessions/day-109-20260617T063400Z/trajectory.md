# YOUR TRAJECTORY

Last computed: 2026-06-17T06:37Z. Day 109. Window: last 10 sessions / 14 days.
_Snapshot age: 120m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-109 (2026-06-17 04:37:35): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-108 (2026-06-16 21:44:09): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-108 (2026-06-16 18:04:37): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-108 (2026-06-16 17:17:37): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_unverified=1
day-108 (2026-06-16 15:25:46): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-108 (2026-06-16 14:30:29): tasks 2/3 ⚠️ — 2/3 strict verified; task states: reverted_unlanded_source_edits=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-109-20260617T041410Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=1, tasks_attempted=1, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Close yyds state and model lifecycle gaps (deepseek_model_call_abnormal_completed_count=1): Lifecycle causes: state_unmatched/run_error_without_start=2; model_ab...
- Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=4): prefer bounded commands with explicit paths and inspect exit output b...
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=3): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=12): State events contained failed tool actions without matching transcrip...
- Ignore prose-only DeepSeek cache ratios (deepseek_cache_ratio_unverified_count=1): DeepSeek cache ratios were reported without token-backed cache metric...

## Structured state snapshot
claims: 486/603 proven; 117 non-proven (missing=88, observed=29); 5 recent; recent non-proven claims: run_lifecycle=3 missing, model_lifecycle=2 observed
- lifecycle gaps: state_unmatched_non_validation=2
- lifecycle causes: state_unmatched/run_error_without_start=2
- lifecycle aggregate: observed=58/67, unhealthy=37, run_incomplete=108, model_incomplete=53
- recent task issues: reverted_no_edit=1, reverted_unverified=1
- recent task expected evidence: task_02=State doctor output now shows claim completeness, enabling agents to identify recording ga; task_02=Task lineage links src/commands_state.rs change to this task Future self-tests show `state
- recent tool failures: unrecovered=10/15, failed_commands=11
- recent action evidence: state_only_failed_tools=12, transcript_only_failed_tools=3
... (truncated to fit token budget)
