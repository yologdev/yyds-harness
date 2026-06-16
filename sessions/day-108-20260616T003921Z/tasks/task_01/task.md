Title: Close state run lifecycle gaps — emit RunCompleted for orphaned runs
Files: src/state.rs
Issue: none
Origin: planner

Objective:
Ensure every RunStarted event eventually has a matching RunCompleted event in the state database, even when the previous run terminated abnormally (panic, SIGTERM, timeout, CI cancellation). When `StateRecorder::init_global` runs and detects an open RunStarted without a matching RunCompleted from a prior session, it should retroactively emit RunCompleted("error") for that orphaned run before starting the new run.

Why this matters:
The trajectory reports `state_run_incomplete_count=1` as the top-ranked graph pressure. The log feedback lesson says: "state run lifecycle was incomplete: state_incomplete/open_after_SessionStarted=1 → emit RunCompleted events for every started run, including timeout and API-error exits." The `state why last-failure` command correctly detects incomplete runs but can't close them — the gap persists in the state database. The RunCompletionGuard (Drop-based) handles clean exits but can't catch SIGKILL, SIGTERM, or process aborts. Every orphaned run is a permanent evidence gap that weakens state-driven evolution decisions.

This is the top-ranked graph pressure and directly impacts DeepSeek harness evidence quality.

Success Criteria:
- On startup, `init_global` checks for a dangling RunStarted event from a prior run that has no matching RunCompleted, and if found, emits RunCompleted("error") with a detail message like "previous run did not complete (orphaned)" before starting the new run.
- The RunCompletionGuard (Drop-based completion for clean exits) is unchanged and continues to work for normal shutdown.
- The fix does not double-emit RunCompleted — clean exits still produce exactly one RunCompleted via the guard.
- `cargo test --lib state` passes.
- `state why last-failure` no longer reports incomplete runs from past sessions (only the current in-progress session).

Verification:
- cargo test --lib state
- cargo test --lib state::tests::run_completion_guard -- --exact
- cargo check
- yyds state why last-failure (run manually after the change to confirm orphaned-run handling)

Expected Evidence:
- Future trajectory reports show `state_run_incomplete_count=0` for sessions after this change.
- The log feedback "state run lifecycle was incomplete" lesson stops recurring.
- state lifecycle gnomes improve: open_after_SessionStarted count drops to zero.

Implementation Notes:
The detection logic should be added near the top of `init_global` (currently around line 307 where RunStarted is appended). Before appending the new RunStarted event, scan the most recent events in the state file for a RunStarted that has no subsequent RunCompleted from the same run_id. If found, emit a RunCompleted("error") with detail "previous run did not complete (orphaned)" using the same run_id.

The scan does not need to be exhaustive — check the last ~20 events for a dangling RunStarted. This is O(1) with respect to total state size.

Key design constraint: the retroactive RunCompleted must use the orphaned run's run_id, not the new run's id. The run_id is in the RunStarted payload. Read it from the dangling event's payload.

If no previous RunStarted exists (fresh state file) or the most recent event is already a RunCompleted, do nothing — there's nothing to close.

Do not modify the RunCompletionGuard or its Drop implementation — the guard handles clean exits correctly. This change only covers abnormal exits where the guard's Drop never runs.
