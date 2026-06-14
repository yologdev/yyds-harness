Title: Close state lifecycle gaps — audit and fix ModelCallCompleted/RunCompleted emission
Files: src/prompt.rs, src/state.rs
Issue: none
Origin: planner

Objective:
Ensure every ModelCallStarted event has a matching ModelCallCompleted, and every
RunStarted has a matching RunCompleted, even on error/early-return paths.

Why this matters:
The harness trajectory shows:
- model_incomplete/open_after_command=1 — ModelCallStarted recorded, tool completed,
  but ModelCallCompleted never emitted
- state_incomplete/open_after_cache_metrics=1 — RunStarted recorded, cache metrics
  emitted, but RunCompleted never emitted
- state_incomplete/open_after_command=1 — RunStarted recorded, command completed,
  but RunCompleted never emitted

These gaps mean state diagnostics, replay, and dashboard projections operate on
incomplete data. Claims about model/run health can't be verified when lifecycle
events are missing.

Success Criteria:
- ModelCallCompleted is recorded in ALL exit paths from handle_prompt_events,
  including when the function panics (via RAII guard on drop)
- RunCompleted is recorded even if run_cli panics after state init but before
  RunCompletionGuard drops (verify the panic hook emits it)
- Existing behavior unchanged: normal completion, ctrl_c interrupt, and
  stream-closed-without-agent-end paths continue to work

Verification:
- cargo test --lib -- prompt::tests -- --test-threads=1
- cargo test --lib -- state::tests -- --test-threads=1
- cargo build
- Manual: verify state tail shows properly paired Started/Completed events

Expected Evidence:
- Lifecycle gap counts (model_incomplete, state_incomplete) decrease to zero in
  subsequent trajectory blocks
- Dashboard claims about lifecycle health improve (proven count increases)
- No regression in existing test suite

Implementation Notes:

### Problem 1: ModelCallCompleted not recorded on panic

In `src/prompt.rs`, `handle_prompt_events` records `ModelCallStarted` at line 768
and records `ModelCallCompleted` in three places: AgentEnd (922), ctrl_c (993),
and after the event loop (1033). However, if the event loop panics between
ModelCallStarted and the after-loop cleanup, ModelCallCompleted is never emitted.

Fix: Create a `ModelCallGuard` struct (similar to `RunCompletionGuard` in state.rs)
that records ModelCallCompleted on Drop with status "panicked" if not already
explicitly completed. Place the guard right after ModelCallStarted.

Suggested implementation pattern:
```rust
struct ModelCallGuard { recorded: bool, model: String, ... }
impl ModelCallGuard {
    fn mark_completed(&mut self) { self.recorded = true; }
}
impl Drop for ModelCallGuard {
    fn drop(&mut self) {
        if !self.recorded {
            record(EventType::ModelCallCompleted, ..., "guard_drop_no_explicit_completion", ...);
        }
    }
}
```

In handle_prompt_events, create the guard after line 771, replace the three
current ModelCallCompleted emission points with guard.mark_completed() calls.
The guard's Drop handles the panic path.

### Problem 2: RunCompleted not recorded on panic after state init

In `src/state.rs`, the `RunCompletionGuard` (line 334) marks "completed" on drop.
But if `run_cli` panics after `state::init_global` but before the prompt loop
starts (e.g., in agent building), the guard drops and marks "completed" even
though the run failed.

The `install_panic_hook` (called at line 1050 in lib.rs) should handle this, but
verify it records a RunCompleted (or relies on the guard's drop). The panic hook
should set the guard's status to "error" or record a separate RunCompleted with
error status.

Fix: In the panic hook (state.rs), call `mark_run_completed_with_error` if the
run hasn't already been marked completed. Or ensure the RunCompletionGuard records
"error" instead of "completed" when dropping during a panic.

### Scope

Keep changes to src/prompt.rs and src/state.rs only. Do not touch lib.rs,
deepseek.rs, or any other files unless a direct dependency emerges during
verification.
