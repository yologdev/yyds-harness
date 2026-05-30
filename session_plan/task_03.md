Title: Fix fixed-path temp dir tests in cli.rs, setup.rs, and prompt_utils.rs
Files: src/cli.rs, src/setup.rs, src/prompt_utils.rs
Issue: none

## Problem

Three more files have fixed-path temp directory tests that can race under parallel execution:
- `src/cli.rs`: 4 instances (`yoyo_test_system_file`, `yoyo_test_sf_override`, `yoyo_test_cli_sf_override`, `yoyo_test_cli_sys_vs_config_file`)
- `src/setup.rs`: 4 instances (`yoyo_test_wizard`, `yoyo_test_xdg_save`, `yoyo_test_xdg_nested`, `yoyo_test_wizard_user_save`)
- `src/prompt_utils.rs`: 1 instance (`yoyo_test_output`)

This is the final batch of the fixed-path temp dir sweep. After Tasks 1 and 2, these are the
remaining files with this anti-pattern.

## What to Do

1. In each file, replace all `std::env::temp_dir().join("yoyo_test_...")` patterns with
   `tempfile::Builder::new().prefix("yoyo_test_").tempdir().unwrap()`.

2. For each test:
   - Replace the fixed path creation with `let tmp_dir = tempfile::Builder::new().prefix("yoyo_test_").tempdir().unwrap();`
   - Use `tmp_dir.path()` (or `.to_path_buf()`) where the test references the path
   - Remove manual `remove_dir_all` cleanup calls (TempDir auto-cleans on drop)
   - Remove pre-test `remove_dir_all` calls (unique names, no conflict)

3. Add `use tempfile;` or the appropriate import if not already present in the test module.

## Important Notes

- `tempfile` crate is already in Cargo.toml.
- Keep the `tmp_dir` binding alive for the whole test scope.
- This completes the sweep: after this task, zero fixed-path temp dirs should remain in the codebase.

## Verification

- `cargo build` passes
- `cargo test` passes (full suite)
- `cargo clippy --all-targets -- -D warnings` clean
- `grep -rn 'temp_dir.*join.*"yoyo_test' src/` returns zero results
