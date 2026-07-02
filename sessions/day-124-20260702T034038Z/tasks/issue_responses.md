# Issue Responses — Day 124

## #51 — Task reverted: Fix `yyds state why last-failure` timeout
**Decision:** implement as Task 01

The implementation was likely correct — it was reverted because the evaluator timed out, not because the code was wrong. The same read-everything-to-timeout pattern was already fixed in `state doctor` and `state crashes` (Day 122). This completes the sweep: add a sampling cap to `build_why_report` so the command completes in under 15 seconds. The task is scoped to ~30 lines to minimize evaluator time.

## #52 — Task reverted: Fix `yyds deepseek cache-report` timeout
**Decision:** implement as Task 02

Same class as #51 and same fate: correct implementation, evaluator timed out. The fix follows the established sampling pattern. Making all three timeout commands (state why, cache-report, eval score) responsive closes the `bash_tool_error=10` pressure signal and restores cost visibility for DeepSeek caching.

## #53 — Task reverted: Make `append_terminal_state_events.py` robust against evaluator-timeout orphaned runs
**Decision:** implement as Task 03, narrowed to a single change

The previous attempt (17+ turns, no implementation landed) tried to do too much: FailureObserved tracking, `closed_by` field, multiple orphan types, full robustness rewrite. The agent did correctly identify the root cause on Turn 19: `lifecycle_for_scope` explicitly skips session runs, so session-scope orphans are never closed by the post-hoc closer.

Task 03 fixes ONLY that gap: detect session-scope runs with RunStarted but no RunCompleted, append RunCompleted with outcome "post_hoc_closed". ~30 lines in the script, one unit test. No refactoring.

## #37 — Add held-out coding eval coverage for DeepSeek harness gnomes
**Decision:** defer — tracking issue, not actionable yet

This is a valid goal but blocked on the evaluator infrastructure working reliably. Adding eval fixtures while the evaluator times out on verification is circular — the fixtures would never be verified. Once the evaluator timeout is resolved (via #53 or otherwise), this becomes a candidate. The fitness score is still "unknown" because the measurement infrastructure isn't trustworthy yet — fixing that is prerequisite.
