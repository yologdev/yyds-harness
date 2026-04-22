Title: Finish safety sweep — remove stale #[allow(dead_code)], harden last production .unwrap() calls
Files: src/repl.rs, src/commands_session.rs, src/format/markdown.rs
Issue: none

## What to do

The codebase has been through a thorough safety sweep (Days 51-53) but three tiny items remain:

### 1. Remove stale `#[allow(dead_code)]` on `CommandResult` (repl.rs:26)

The `CommandResult` enum at line 26 of `repl.rs` has `#[allow(dead_code)]` but all four variants (`Continue`, `Quit`, `SendToAgent`, `NotACommand`) are actively used. Remove the annotation — it's a lie.

### 2. Harden `stash.pop().unwrap()` (commands_session.rs:587)

In `handle_stash_pop`, line 587 does `let entry = stash.pop().unwrap()`. This is after a check that `stash.is_empty()` returns false, so logically safe, but `.unwrap()` on a `Vec::pop()` can still panic if there's a race or logic bug. Replace with a match/if-let that returns an error message instead of panicking.

### 3. Harden `chars().next().unwrap()` (format/markdown.rs:565)

Line 565 does `let first = no_spaces.chars().next().unwrap()`. Check the surrounding context — if `no_spaces` is guaranteed non-empty by prior logic, consider adding a guard anyway (or an early return). If it could be empty, add a proper check.

### 4. Add tests

- Add a test verifying `CommandResult` variants compile without the `dead_code` annotation (implicit — if it builds without warnings, the test is the build itself)
- Add a test for the stash pop edge case (empty stash after check)
- Add a test for the markdown renderer with an empty string input if one doesn't exist

### Verification

```bash
cargo build && cargo test && cargo clippy --all-targets -- -D warnings
```

All must pass. If clippy shows warnings about unused variants after removing `#[allow(dead_code)]`, that means a variant IS actually dead and should be investigated (but all four are used based on grep).
