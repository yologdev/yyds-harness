Title: Fix flaky watch test — isolate global WATCH_COMMANDS state in tests
Files: src/watch.rs
Issue: none

## Context

The CI trajectory shows `handle_watch_bare_sets_lint_and_test` appearing 5 times in recent
failure fingerprints. The test calls `handle_watch("/watch")` which:
1. Calls `detect_watch_all_phases()` (probes the filesystem for Cargo.toml etc.)
2. Sets the global `static WATCH_COMMANDS: RwLock<Vec<String>>`
3. Asserts the watch command was set correctly

The flakiness comes from test parallelism: multiple tests read/write the global `WATCH_COMMANDS`
concurrently. Tests like `handle_watch_bare_sets_lint_and_test` and
`detect_watch_all_phases_returns_separate_commands` both manipulate the same global state.

## What to do

1. Add a `#[serial]` or mutex-based serialization for watch tests that touch `WATCH_COMMANDS`:
   - Create a `static WATCH_TEST_LOCK: Mutex<()>` in the test module
   - Each test that calls `set_watch_command`, `set_watch_commands`, `clear_watch_command`,
     `handle_watch`, or reads `get_watch_command`/`get_watch_commands` should acquire this lock
     at the start
   - This is the minimal fix — no new dependencies needed

2. Ensure every test that sets watch commands also clears them at the end (some already do,
   verify all do).

3. Add a comment explaining why the lock exists.

## Specific tests to fix

Look at ALL tests in the `#[cfg(test)] mod tests` block in `watch.rs` that touch the global
watch state. Each one needs the test lock. The pattern:

```rust
fn test_something() {
    let _lock = WATCH_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // ... test body ...
    clear_watch_command(); // cleanup
}
```

## Important notes

- Do NOT add external dependencies (like `serial_test` crate) — use a simple Mutex
- The fix must not change any non-test code
- Keep existing test logic unchanged — just add serialization
- Run `cargo test -- watch::tests` multiple times to verify no flakiness
