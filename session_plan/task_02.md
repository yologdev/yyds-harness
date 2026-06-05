Title: Fix fragile detect_watch_all_phases test to use temp directory
Files: src/watch.rs
Issue: none

## What

The `detect_watch_all_phases_returns_separate_commands` test (line ~2025 in watch.rs) depends on
the CWD being a Rust project. This is the same fragility pattern that caused the CI flake in
`handle_watch_bare_sets_lint_and_test` (fixed in Day 96 session 1 by parameterizing with a 
directory argument). The trajectory shows this test panicked in CI once already.

## How

1. Replace the call to `detect_watch_all_phases()` (which reads CWD) with 
   `detect_watch_all_phases_for_dir(&tmp)` where `tmp` is a temp directory with a `Cargo.toml`.

2. This follows the exact same pattern as `detect_watch_all_phases_for_dir_rust_project` 
   (which already exists at line ~2067) but tests the 2-phase return shape more thoroughly.

3. Remove the `#[serial]` attribute from this test since it no longer depends on shared state
   (CWD). The `detect_watch_all_phases_for_dir` function takes a path argument.

4. Clean up the temp directory after the test.

## Verification

- `cargo test detect_watch_all_phases_returns_separate_commands` passes
- `cargo test` all pass
- `cargo clippy --all-targets -- -D warnings` clean
