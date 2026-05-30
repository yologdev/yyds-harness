Title: Fix fixed-path temp dir tests in commands_project.rs
Files: src/commands_project.rs
Issue: none

## Problem

`commands_project.rs` has 16 tests using fixed temp paths like `yoyo_test_node`, `yoyo_test_rust`,
`yoyo_test_python`, etc. These are all potential race conditions under parallel test execution —
the same class of bug that caused flaky CI in 4 prior sessions.

## What to Do

1. Replace all 16 fixed temp path patterns (`std::env::temp_dir().join("yoyo_test_...")`) with
   `tempfile::TempDir` or `tempfile::Builder::new().prefix("yoyo_test_").tempdir().unwrap()`.

2. For each test:
   - Replace `let tmp = std::env::temp_dir().join("yoyo_test_...");` with
     `let tmp_dir = tempfile::Builder::new().prefix("yoyo_test_").tempdir().unwrap();`
     `let tmp = tmp_dir.path().to_path_buf();`
   - Remove the `let _ = std::fs::remove_dir_all(&tmp);` cleanup line at end (TempDir handles it)
   - Remove the `let _ = std::fs::remove_dir_all(&tmp);` setup line at start (unique name, no conflict)

3. The rest of the test logic stays the same — only the temp directory creation and cleanup changes.

## Important Notes

- `tempfile` crate is already in Cargo.toml — no dependency changes needed.
- Keep the `tmp_dir` binding alive for the whole test (don't let it drop early, which would delete the dir).
- Some tests may use `tmp` as a `PathBuf` — ensure `tmp_dir.path()` provides a compatible `&Path`.

## Verification

- `cargo build` passes
- `cargo test commands_project` passes
- `cargo clippy --all-targets -- -D warnings` clean
