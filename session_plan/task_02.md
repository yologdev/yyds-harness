Title: Add unit tests for commands_update.rs — the only untested file over 100 lines
Files: src/commands_update.rs
Issue: none

## Context

`commands_update.rs` is 422 lines with ZERO tests — the only source file over 100 lines without any test coverage. It handles the critical `/update` self-update command, which downloads and replaces the running binary. The pure helper functions in this file are straightforward to test without making network calls.

## What to do

1. Add a `#[cfg(test)] mod tests` block at the bottom of `src/commands_update.rs`.

2. Write tests for **`platform_asset_name(os, arch)`** — this is a pure function mapping (os, arch) pairs to asset file names:
   - `("linux", "x86_64")` → `Some("yoyo-x86_64-unknown-linux-gnu.tar.gz")`
   - `("macos", "x86_64")` → `Some("yoyo-x86_64-apple-darwin.tar.gz")`
   - `("macos", "aarch64")` → `Some("yoyo-aarch64-apple-darwin.tar.gz")`
   - `("windows", "x86_64")` → `Some("yoyo-x86_64-pc-windows-msvc.zip")`
   - Unsupported combinations (e.g., `("linux", "arm")`, `("freebsd", "x86_64")`) → `None`

3. Write tests for **`is_cargo_dev_build()`** — this checks whether the binary was built via `cargo run` (development mode) vs installed:
   - In test environment (running via `cargo test`), it should return `true`
   - Test the logic path: the function checks `env::current_exe()` and path patterns

4. Write tests for **`current_binary_path()`** — returns the path to the running binary:
   - Should return `Ok(path)` where path exists
   - The path should be a valid file path

5. Write tests for **`extract_archive()`** edge cases (if the function signature allows it without actual file I/O, use temp dirs):
   - If the function can be tested with a temp directory containing a mock archive, add a test
   - If it requires actual tar/zip extraction, test error cases (nonexistent archive path, nonexistent extract dir)

6. Write tests for any **URL construction or version parsing** logic in the file:
   - Check `fetch_latest_release` error handling (without actual network calls — just verify the function signature and error paths)
   - Check version comparison paths if any exist in this file

Target: at least 15 test functions covering the pure/deterministic functions. Skip tests that would require network calls or actual binary replacement.

## Verification

```bash
cargo test commands_update -- --nocapture
cargo clippy --all-targets -- -D warnings
```
