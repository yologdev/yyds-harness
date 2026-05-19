Title: Fix flaky context.rs tests — add #[serial] to set_current_dir tests
Files: src/context.rs
Issue: none

## Problem

6 tests in `context.rs` use `std::env::set_current_dir()` without `#[serial]`, causing
intermittent failures when tests run in parallel. This is the **#1 recurring CI error** in
the trajectory (5 instances in the recent window). The same bug class was already fixed in
`watch.rs` (Day 79) and `commands_config.rs` (Day 77).

## What to do

1. Add `use serial_test::serial;` to the test module in `context.rs` (it's already a
   dev-dependency in Cargo.toml).

2. Add `#[serial]` attribute to every test function that calls `std::env::set_current_dir()`.
   There are 6 such tests — search for `set_current_dir` in the test module to find them all.
   The test names include:
   - `test_load_project_context_with_yoyo_md`
   - `test_load_project_context_with_claude_md`
   - `test_load_project_context_prefers_yoyo_md`
   - `test_load_project_context_with_agents_md`
   - `test_load_copilot_instructions_file`
   - `test_load_cursorrules_file`

3. Run `cargo test context::tests` multiple times (at least 3 times) to verify no flakiness.

4. Run full `cargo test` to confirm nothing else breaks.

## Why this matters

This directly protects evolution session reliability. Every flaky test is a session risk —
the trajectory shows this exact pattern caused 5 CI failures in the recent window. Mechanical
fix, high impact.
