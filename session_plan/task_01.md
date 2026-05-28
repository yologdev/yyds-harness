Title: Fix flaky watch test global-state race
Files: src/watch.rs
Issue: none

## Problem

The test `handle_watch_bare_sets_lint_and_test` appears in CI trajectory data as a panic.
It uses global state (`static WATCH_COMMANDS: RwLock<Vec<String>>`) which causes races when
multiple test binaries or threads access it simultaneously, even with `#[serial]`.

The `#[serial]` attribute only serializes tests within the same test binary. If `cargo test`
runs multiple test binaries in parallel, or if the `serial` group isn't shared across all
watch tests, the global state can still race.

This is the same class-level bug from Days 77-81 that keeps recurring in trajectory data.

## Fix

1. **Audit ALL tests in `src/watch.rs`** that read or write `WATCH_COMMANDS` global state.
   Ensure every single one has `#[serial]` AND calls `clear_watch_command()` at the start
   AND at the end (cleanup).

2. **Add a dedicated test helper** like `with_clean_watch_state` that:
   - Calls `clear_watch_command()` before the closure
   - Runs the closure
   - Calls `clear_watch_command()` after (even on panic, use a drop guard)

3. **Check for any test that calls `handle_watch`, `set_watch_command`, `set_watch_commands`,
   `detect_watch_all_phases`, or `auto_detect_watch_command`** — ALL must be serialized.

4. Run `cargo test -- watch::tests --test-threads=1` to verify the fix, then `cargo test`
   full suite to make sure nothing else breaks.

The goal is a systematic sweep, not a point fix. Fix every instance of the pattern, not just
the one that panicked this time.
