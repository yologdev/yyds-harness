Title: Fix architect mode test flakiness — add #[serial] to global state tests
Files: src/commands_config.rs
Issue: none

## Problem

The CI trajectory shows 1× `assertion failed: is_architect_mode()` in the recent window, and this is a recurring CI error pattern (4× test failures total). The root cause: `ARCHITECT_MODE` is a global `AtomicBool` that multiple test functions read/write. When `cargo test` runs tests in parallel, one test's `set_architect_mode(true)` bleeds into another test's assertion.

## Solution

The `serial_test` crate is already in `Cargo.toml`. Add `use serial_test::serial;` and `#[serial]` to every test in `commands_config.rs` that touches global mutable state:

1. Find all tests that call `set_architect_mode`, `is_architect_mode`, `set_teach_mode`, or `is_teach_mode` — these all use global `AtomicBool` state.
2. Add `#[serial]` attribute to each of those tests.
3. Also check for any other global `AtomicBool` or `static` mutable state in `commands_config.rs` tests and serialize those too.

The `#[serial]` attribute ensures these tests run one-at-a-time, not in parallel, preventing the race condition.

## Verification

```bash
cargo test --lib commands_config -- --test-threads=1  # baseline
cargo test --lib commands_config  # parallel — should now pass reliably
cargo test  # full suite
cargo clippy --all-targets -- -D warnings
```

Run the parallel test multiple times to confirm no flakiness:
```bash
for i in $(seq 1 5); do cargo test --lib commands_config 2>&1 | tail -1; done
```
