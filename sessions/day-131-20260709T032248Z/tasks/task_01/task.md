Title: Recognize SessionStarted as lifecycle start in orphan-run detection
Files: src/state.rs
Issue: #83
Origin: planner (refined from harness-seed)

Evidence:
- Trajectory state_run_incomplete_count=1 with open_after_SessionStarted=1
- Graph-derived next-task pressure #1: "Close yyds state and model lifecycle gaps"
- Assessment confirms: "close_orphaned_run_if_needed only scans for RunStarted events, but SessionStarted is emitted later. A session that crashes after SessionStarted but before RunCompleted leaves an orphaned run."
- Issue #83 was reverted — previous attempt was too broad/investigative and the implementation agent never landed code
- Log feedback corrected lesson: "state run lifecycle was incomplete: state_incomplete/open_after_SessionStarted=1 → emit RunCompleted events for every started run"

Edit Surface:
- src/state.rs: the backward scan loop in `close_orphaned_run_if_needed` (fn at line 315)

Verifier:
- cargo test state::tests::close_orphaned_run -- --nocapture

Fallback:
- If SessionStarted events don't carry a run_id field, or if the existing tests already cover this case and the remaining gap is purely in Python script classification, write task_01_obsolete.md

Objective:
Teach `close_orphaned_run_if_needed` to recognize `SessionStarted` as a lifecycle start event alongside `RunStarted`, so sessions that emit SessionStarted but crash before RunCompleted get retroactively closed.

Why this matters:
The run lifecycle tracking has a blind spot: `close_orphaned_run_if_needed` only scans for `RunStarted` events, but `SessionStarted` is emitted later (in `record_session_started` → `lib.rs`). A session that crashes after SessionStarted but before RunCompleted leaves an orphaned run that the retroactive closure logic never detects. This is the #1 graph-derived pressure item.

Success Criteria:
- `close_orphaned_run_if_needed` recognizes `SessionStarted` as a lifecycle start event when scanning backward
- When SessionStarted is found without a matching RunCompleted, the function uses its run_id to emit a retroactive RunCompleted
- Existing tests pass; SessionStarted-based orphan detection is covered by existing or new test
- After fix, the next trajectory/dashboard snapshot shows open_after_SessionStarted disappears

Verification:
- cargo build && cargo test --lib -- state::tests -- --nocapture
- cargo test state::tests::close_orphaned_run -- --nocapture
- cargo test state::tests::orphaned_run -- --nocapture

Expected Evidence:
- Future structured state snapshots show state_run_incomplete_count moving to 0
- "Close yyds state and model lifecycle gaps" drops off graph-derived pressure
- No new open_after_SessionStarted instances appear

Implementation Notes:
The fix is surgical — roughly 5-8 lines changed in the backward scan loop. When scanning backward through events looking for lifecycle start events (currently only RunStarted), also recognize SessionStarted. The loop currently at line ~348 checks if the event type is RunStarted; add SessionStarted as an alternative. When a SessionStarted is found without a matching RunCompleted, extract its run_id from the event payload (same as RunStarted events) and use it for the retroactive closure.

The existing RunCompleted emission logic (after the scan completes) can be reused as-is — just ensure the run_id extracted from SessionStarted events is compatible with the existing `append_event_with_projection` call.

Keep the change MINIMAL — this is a recognition fix, not a refactor. The function should still bail out on RunCompleted and still verify no RunCompleted exists for the detected run_id before emitting one.

Previous attempt (Day 130) failed because the implementation agent did too much exploration and never landed code. This time the task is narrowly scoped: find the backward scan loop, add `|| event_type == EventType::SessionStarted` next to the RunStarted check, and ensure the run_id extraction works. That's it.
