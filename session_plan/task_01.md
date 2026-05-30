Title: Fix flaky temp dir tests in commands_info.rs and commands_session.rs
Files: src/commands_info.rs, src/commands_session.rs
Issue: none

## Problem

`test_compute_self_written_temp_repo` uses a fixed path `yoyo_test_self_written` that races under
parallel test execution. This is the 5th instance of the exact same bug class (Days 77, 79, 80, 81
fixed others). The assessment flagged it as the known flaky test causing CI noise.

`commands_session.rs` has 3 more fixed-path tests (`yoyo_test_autosave`, `yoyo_test_continue_path`,
`yoyo_test_continue_fallback`). `commands_info.rs` has 2 (`yoyo_test_no_git_sw`, `yoyo_test_self_written`).

## What to Do

1. In `src/commands_info.rs`, replace both fixed temp paths with `tempfile::TempDir::new().unwrap()`
   (or `tempfile::Builder::new().prefix("yoyo_test_").tempdir().unwrap()`). The `tempfile` crate
   is already in `Cargo.toml`. Remove the manual `remove_dir_all` cleanup since `TempDir` auto-cleans
   on drop.

2. In `src/commands_session.rs`, replace all 3 fixed temp paths with `tempfile::TempDir` similarly.

3. Run `cargo test` to verify the flaky test now passes reliably. Run the specific tests a few times
   to confirm no races:
   ```
   cargo test test_compute_self_written_temp_repo -- --test-threads=1
   cargo test test_compute_self_written_temp_repo
   ```

## Verification

- `cargo build` passes
- `cargo test` passes (no flaky failures)
- `cargo clippy --all-targets -- -D warnings` clean
