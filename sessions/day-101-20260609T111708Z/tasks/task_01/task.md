Title: Wire panic hook into diagnostic error stash

Files: src/state.rs

Issue: none

Origin: planner

Objective:
  Ensure that when a Rust panic kills the process, the `RunCompleted` event
  carries the panic message and location as `error_detail` — not just a generic
  "exit code 1" message. This closes the biggest observability gap in crash
  diagnosis: 10 crashes this session alone, zero with diagnostic payloads.

Why this matters:
  - The crash reporter (`stash_diagnostic_error` / `take_diagnostic_error`) exists
    but is only wired into ONE location (`src/lib.rs` line 1032, state init failure).
  - The panic hook (`install_panic_hook` in `src/state.rs` line 30-55) already
    records a `FailureObserved` event but does NOT call `stash_diagnostic_error`.
  - When a panic occurs, the process exits via `exit_with_state(code)` →
    `mark_run_completed_with_error(msg)` → `take_diagnostic_error()` returns `None`.
  - The `RunCompleted` payload has `error: "exit code 1"` with no `error_detail`.
  - `state crashes` shows 10 crashes with `diagnostic_key=no` — invisible failures.
  - This is a one-line fix: add `stash_diagnostic_error(...)` in the panic hook
    after the existing `record(...)` call. Also remove the `#[allow(dead_code)]`
    attribute since `stash_diagnostic_error` will now have two real callers.

Success Criteria:
  - Panic hook calls `stash_diagnostic_error` with panic message + location
  - `stash_diagnostic_error` no longer has `#[allow(dead_code)]`
  - If a thread panics with message "test panic at foo.rs:42", then
    `take_diagnostic_error()` returns Some("rust_panic: test panic at foo.rs:42")
  - New test verifies this behavior

Verification:
  - cargo build (must compile clean)
  - cargo test (must pass all tests including new one)

Expected Evidence:
  - `state crashes` after a panic-based crash should show `diagnostic_key=yes`
    with the panic message in `error_detail`
  - No more RunCompleted(error) events with empty error_detail when a panic occurred

---

## Implementation Plan

### Step 1: Modify `install_panic_hook` in `src/state.rs`

Inside the panic hook closure (line 33-53), after the existing `record(EventType::FailureObserved, ...)` call, add:

```rust
// Stash the diagnostic so RunCompleted includes the actual failure reason
stash_diagnostic_error(&format!("rust_panic: {msg} at {location}"));
```

The hook should look like (exact code after the `record(...)` call):

```rust
            record(EventType::FailureObserved, Actor::Harness, payload);
            // Also stash for RunCompleted error_detail
            stash_diagnostic_error(&format!("rust_panic: {msg} at {location}"));
            prev_hook(info);
```

### Step 2: Remove `#[allow(dead_code)]`

The `stash_diagnostic_error` function (line 70-71) currently has `#[allow(dead_code)]`
because it only had one caller in `src/lib.rs` (a different crate). Now it has
two callers — the panic hook in the same module and `src/lib.rs` in the binary
crate — so the warning is still possible since the library crate's dead_code
analysis sees only the panic hook caller. Keep `#[allow(dead_code)]` if the
compiler still warns, but remove it if the warning goes away.

Actually: `src/lib.rs` is the library crate root, and `stash_diagnostic_error` is
in `src/state.rs` which is part of the same library crate. The caller at
`src/lib.rs:1032` (`state::stash_diagnostic_error(...)`) is in the same crate,
so there are already two callers in the same crate: the panic hook (via `stash_diagnostic_error`
at function scope) and `lib.rs` (via `state::stash_diagnostic_error`). The
`#[allow(dead_code)]` was there from before `lib.rs` was a caller. But wait —
`lib.rs` calls it via `state::stash_diagnostic_error` which is the public API,
so the compiler won't complain about dead code. The `#[allow(dead_code)]` is
actually suppressing the warning for the *private* function body being unused
from within the module. After adding the panic hook caller (inside `state.rs`
itself), the function has a module-internal caller, so dead_code should be
resolved. Remove `#[allow(dead_code)]`.

### Step 3: Add a test

Add a test in `src/state.rs` that:
1. Spawns a thread that panics with a known message
2. Catches the panic via `std::panic::catch_unwind`
3. Asserts that `take_diagnostic_error()` returns the expected message

Something like:

```rust
#[test]
fn test_panic_hook_stashes_diagnostic_error() {
    // Clear any prior stashed error
    let _ = take_diagnostic_error();
    
    let result = std::panic::catch_unwind(|| {
        panic!("intentional test panic");
    });
    assert!(result.is_err());
    
    let diag = take_diagnostic_error();
    assert!(diag.is_some(), "panic hook should have stashed diagnostic error");
    let diag = diag.unwrap();
    assert!(diag.contains("rust_panic"), "should contain 'rust_panic' prefix, got: {diag}");
    assert!(diag.contains("intentional test panic"), "should contain panic message, got: {diag}");
}
```

Note: This test will trigger the panic hook which calls `record(...)`. If the
global state recorder is not initialized, `record` is fail-soft (it checks for
initialization internally). The test should work with or without state init.

### Step 4: Verify

```bash
cargo build
cargo test --bin yyds -- --test-threads=1
cargo test --test integration -- --test-threads=1
cargo clippy --all-targets -- -D warnings
```

### Risks
- The panic hook runs in an unstable state; calling `stash_diagnostic_error`
  (which just sets a thread-local `RefCell`) is safe — no allocation that could
  fail during unwind.
- The `#[allow(dead_code)]` removal: if the compiler still warns (unlikely given
  the module-internal caller), add it back with a comment explaining why.
