# YOUR TRAJECTORY

Last computed: 2026-06-18T04:09Z. Day 110. Window: last 10 sessions / 14 days.
_Snapshot age: 264m (fresh) — reliable ✓_

## Recent session outcomes (newest 6 of 10)
day-109 (2026-06-17 23:44:22): tasks 3/3 ✅ — 3/3 strict verified; build OK, tests OK
day-109 (2026-06-17 20:46:55): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
day-109 (2026-06-17 18:46:07): tasks 1/2 ⚠️ — 1/2 strict verified; task states: reverted_no_edit=1
day-109 (2026-06-17 17:25:01): tasks 1/1 ✅ — 1/1 strict verified; build OK, tests OK
day-109 (2026-06-17 12:37:13): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_no_edit=1
day-109 (2026-06-17 06:50:37): tasks 0/1 ⚠️ — 0/1 strict verified; task states: reverted_no_edit=1
... 4 older session outcome(s) omitted

## Evo readiness
- latest day-109-20260617T230228Z: classification=verified_success, can_drive_evolution=true
- evidence: provider_error_count=0, selected_task_count=3, tasks_attempted=3, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0
- action: use this readiness evidence to select the next concrete, verifiable task

## Graph-derived next-task pressure
- Reconcile transcript-only tool failures (transcript_only_failed_tool_count=2): Recent transcripts contained failed tool actions absent from state ev...
- Reconcile state-only tool failures (state_only_failed_tool_count=13): State events contained failed tool actions without matching transcrip...
- Recover failed tool actions before scoring (tool_error_count=1): Failed tool actions were present in session evidence; inspect the dom...
- Reduce successful-task turn overhead (max_task_turn_count=25): A verified task still used many turns, suggesting discovery or verifi...
- Ignore prose-only DeepSeek cache ratios (deepseek_cache_ratio_unverified_count=1): DeepSeek cache ratios were reported without token-backed cache metric...

## GitHub Actions log feedback
latest score=0.9063 confidence=1.0 recurring_failures=0 state_capture=1.0 provider_error_count=0 provider_blocked_session_count=0 task_success_rate=1.0 task_spec_quality_score=1.0
Corrected top lessons for next run:
- failed tool actions were recovered from transcripts -> inspect failed tool calls and add prompt/tool guards for the dominant failure class
Historical repeated across prior log feedback (context only; corrected lessons are current pressure):
- 2x error: could not compile `yoyo-ds-harness` (lib) due to 1 previous error

## Structured state snapshot
claims: 538/657 proven; 119 non-proven (missing=89, observed=30); 2 recent; recent non-proven claims: assessment_artifact=1 observed, run_lifecycle=1 missing
- lifecycle aggregate: observed=64/73, unhealthy=38, run_incomplete=109, model_incomplete=53
- recent task issues: reverted_no_edit=2
... (truncated to fit token budget)
