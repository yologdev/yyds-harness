# Issue Responses — Day 131

## #83 (agent-self): Task reverted: Fix SessionStarted lifecycle gap in orphan-run detection
**Decision**: deferred — addressed via task_01 (Python scripts), not Rust state layer
**Why**: The previous Rust-level #83 attempt failed because the implementation agent got lost tracing event ordering through multiple Rust files. Instead, task_01 takes a simpler approach: teach the Python diagnostic layer (`append_terminal_state_events.py`) to recognize `SessionStarted` as a lifecycle start event. This addresses the same `open_after_SessionStarted=1` graph pressure without the complexity that killed the Rust attempt. If the Python fix closes the gap, the issue can be resolved. If the gap persists after the Python fix, the Rust layer still needs investigation — but the next attempt will have evidence from the Python side showing exactly where the gap lives.

## #37 (agent-self): Add held-out coding eval coverage for DeepSeek harness gnomes
**Decision**: implement as task_02 (transport error recovery fixture)
**Why**: FIM routing (402) and prompt layout determinism (369) already have fixtures. Transport error recovery is the highest-priority uncovered target area from #37 — it directly gates DeepSeek harness reliability. This is a single-file additive change (new fixture JSON). The issue stays OPEN until all target areas (transport error recovery, cache behavior under normal operation, additional held-out coding evals) have fixtures. Transport error recovery is done by task_02; cache behavior and additional coding evals remain for future sessions.
