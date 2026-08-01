Title: Close model call lifecycle in panic path so FailureObserved doesn't leave orphaned ModelCallStarted
Files: src/state.rs, src/prompt.rs
Issue: none
Origin: planner

Evidence:
- trajectory shows `deepseek_model_call_unmatched_completed_count=2` with `state_unmatched/open_after_FailureObserved=2`
- assessment: "Model call lifecycle gaps — state events show runs where FailureObserved closes a run but ModelCallCompleted is missing"
- root cause in src/state.rs `install_panic_hook()` (line 60): records `FailureObserved` + `mark_run_completed_with_error("rust_panic")` but does NOT record `ModelCallCompleted` for in-flight model calls
- root cause in src/prompt.rs (line 773): `model_call_id` is a local variable in `handle_prompt_events`, invisible to the panic hook
- Ctrl+C interrupt path (prompt.rs line 1029-1052) already correctly records ModelCallCompleted — only the panic path is broken
- src/state.rs already has a `thread_local!` block (line 18-28) with `Cell` types — the pattern exists

Edit Surface:
- src/state.rs: add `CURRENT_MODEL_CALL_ID` thread-local, public set/clear fns, record ModelCallCompleted in panic hook
- src/prompt.rs: call set/clear around model call lifecycle

Verifier:
- cargo build
- cargo test --bin yyds -- --test-threads=1

Fallback:
- If the test suite already covers panic-hook lifecycle closure and tests pass without changes, write an obsolete note. Do not add redundant tracking.
- If `CURRENT_MODEL_CALL_ID` set/clear calls introduce a borrow or ownership conflict with the existing prompt.rs event loop, narrow scope to only fix the panic-hook side (record ModelCallCompleted with a generic "unknown" model_call_id and "interrupted" status) and document the limitation.

Objective:
Ensure that when the process panics, any in-flight model call is properly closed with a `ModelCallCompleted(status="interrupted", detail="rust_panic")` event before `FailureObserved` is recorded, so the model call lifecycle is always balanced.

Why this matters:
The `deepseek_model_call_unmatched_completed_count=2` gnome directly degrades state evidence quality. When model calls appear unmatched, the state analysis scripts can't trust the lifecycle data. This erodes assessment quality and task selection. The fix is small (thread-local cell + two set/clear call sites) and closes a known gap between the panic path and the Ctrl+C interrupt path — the Ctrl+C path already handles this correctly.

Success Criteria:
- After a Rust panic, state events include both `ModelCallCompleted` (with status "interrupted") and `FailureObserved` for the same run
- `deepseek_model_call_unmatched_completed_count` does not increase from genuine panic-induced gaps
- Normal (non-panic) model call lifecycle is unchanged — ModelCallStarted/Completed pairs are recorded exactly as before
- All existing tests pass

Verification:
- cargo build
- cargo test --bin yyds -- --test-threads=1
- cargo test --test integration -- --test-threads=1

Expected Evidence:
- Future trajectory snapshots show reduced or zero `deepseek_model_call_unmatched_completed_count` from panic-induced gaps
- State events after a panic show ModelCallCompleted preceding FailureObserved within the same run
- Log feedback no longer reports "model call lifecycle incomplete" for runs that ended in a Rust panic

Implementation Notes:

The fix has three parts, all in existing files:

### Part 1: src/state.rs — Add thread-local tracker

Add to the existing `thread_local!` block (line 18-28):
```rust
static CURRENT_MODEL_CALL_ID: Cell<Option<String>> = const { Cell::new(None) };
```

Add two public functions (near the existing `store_run_error` / `take_diagnostic_error` helpers):
```rust
pub fn set_current_model_call_id(id: String) {
    CURRENT_MODEL_CALL_ID.with(|c| c.set(Some(id)));
}

pub fn clear_current_model_call_id() {
    CURRENT_MODEL_CALL_ID.with(|c| c.set(None));
}
```

### Part 2: src/state.rs — Close model call in panic hook

In `install_panic_hook()` (line 38-68), before `record(EventType::FailureObserved, ...)`:
```rust
// Close any in-flight model call so the lifecycle is balanced
if let Some(ref model_call_id) = CURRENT_MODEL_CALL_ID.with(|c| c.take()) {
    record(
        EventType::ModelCallCompleted,
        Actor::Yoyo,
        json!({
            "model_call_id": model_call_id,
            "model": "",
            "status": "interrupted",
            "detail": "rust_panic",
            "input_tokens": 0,
            "output_tokens": 0,
            "thinking_observed": false,
        }),
    );
}
```

### Part 3: src/prompt.rs — Set/clear current model call ID

After generating `model_call_id` (line 779, before the `record(ModelCallStarted, ...)` call on line 780), add:
```rust
crate::state::set_current_model_call_id(model_call_id.clone());
```

After every `record(ModelCallCompleted, ...)` call (lines 954, 1038, and any fallback at ~1080), add:
```rust
crate::state::clear_current_model_call_id();
```

There are three ModelCallCompleted sites to update:
1. Normal AgentEnd path (around line 970, after `model_call_terminal_recorded = true`)
2. Ctrl+C interrupt path (around line 1052, after `model_calls_completed += 1`)
3. Fallback after loop ends (search for the other `model_call_terminal_recorded` check around line 1080)

Each site needs `crate::state::clear_current_model_call_id();` after the ModelCallCompleted record call.

The `clear_current_model_call_id()` call is idempotent (setting Cell to None is safe to repeat), so adding it at all three sites is correct even if only one fires per invocation.
