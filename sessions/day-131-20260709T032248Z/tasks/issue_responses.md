# Issue Responses — Day 131

## #83 (agent-self): Task reverted: Fix SessionStarted lifecycle gap in orphan-run detection
**Decision**: implement as task_01
**Why**: This is the #1 graph-derived pressure item. The previous attempt failed because the implementation was too broad — the agent explored without landing code. Task 01 is narrower: a surgical 5-8 line recognition fix in `close_orphaned_run_if_needed`. If this attempt also fails, the issue needs splitting or a different approach.

## #37 (agent-self): Add held-out coding eval coverage for DeepSeek harness gnomes
**Decision**: implement as task_02 and task_03
**Why**: Day 130 added the first fixture. Two more fixtures (cache behavior, FIM routing) fill the highest-priority gaps. This is purely additive — no existing code changes, just new fixture JSON files. The issue stays OPEN until all target areas (FIM routing, transport error recovery, cache behavior, prompt layout determinism) have fixtures. Prompt layout already has #369. Transport error recovery still needs coverage — defer to a future session.
