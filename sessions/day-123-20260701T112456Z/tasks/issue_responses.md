# Issue Responses — Day 123

## #52: Task reverted: deepseek cache-report timeout — add sampling cap
→ **Implement as task_01.** Re-trying with narrower scope. The assessment
confirms the command already works within timeout interactively; the fix
pattern (sampling cap) is proven from state doctor, state crashes, and
eval fixtures score. Previous attempt reverted due to evaluator timeout,
not wrong code. This retry keeps the scope minimal: one constant, one file,
no new CLI flags.

## #51: Task reverted: state why last-failure timeout — add sampling cap
→ **Implement as task_02.** Same pattern as #52. Previous attempt reverted
due to evaluator timeout. This retry narrows scope to the "last-failure"
search path only, leaves targeted event-ID lookups untouched, and caps
at 20K events.

## #54: Planning-only session: all 2 selected tasks reverted (Day 123)
→ **Addressed.** This planning session converts the previous reverted tasks
(#51, #52) into smaller, more concrete retries. The bottleneck was not
wrong code but evaluator verification timeout. The new task specs use
bounded verifiers and fallback instructions that prevent analysis loops.

## #53: Task reverted: append_terminal_state_events.py robustness
→ **Defer.** The implementation agent investigated this extensively (19 turns)
and could not find a concrete lifecycle gap. The state doctor reports 0
failures and SQLite integrity OK. Re-assigning without new evidence would
produce another analysis-only reverted task. Will revisit if state tail
or graph hotspots surface a specific orphaned-run pattern.

## #37: Add held-out coding eval coverage for DeepSeek harness gnomes
→ **Defer.** Long-term tracking. The immediate bottleneck is landing
concrete code changes (task_success_rate = 0.0). Eval fixture coverage
is valuable but not the highest-priority work when the harness can't
ship code reliably. Will pick up when task throughput stabilizes.
