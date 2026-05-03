Title: Fix flaky destructive_guard test — eliminate process-global CWD race
Files: src/git.rs
Issue: #364

## Problem

`destructive_guard_allows_destructive_in_temp_dir` uses `std::env::set_current_dir()` which is
process-global. When `cargo test` runs tests in parallel, another test can observe the temp-dir CWD
and incorrectly allow a destructive command, or this test can observe a restored CWD and incorrectly
block. This caused a real CI failure in the skill-evolve workflow (run 25146860447).

## Fix

1. Change the `destructive_guard` function signature to accept an explicit `cwd: &Path` parameter
   instead of calling `std::env::current_dir()` internally:
   ```rust
   fn destructive_guard<'a>(args: &'a [&'a str], cwd: &Path) -> Option<&'a str> {
   ```

2. Update the call site in `run_git()` to pass `std::env::current_dir().ok()` and handle the
   Option appropriately. The guard should only block when cwd is known AND matches the project root.

3. Update ALL existing tests for `destructive_guard`:
   - `destructive_guard_blocks_known_bad_commands_in_project_root` — pass `Path::new(env!("CARGO_MANIFEST_DIR"))` as cwd
   - `destructive_guard_allows_destructive_in_temp_dir` — pass `std::env::temp_dir()` as cwd, **no more set_current_dir**
   - `destructive_guard_empty_args` — pass any path
   - `destructive_guard_list_covers_original_incident` — no change needed (doesn't call the function)

4. Verify no other code calls `destructive_guard` directly (search the codebase).

## Verification

- `cargo test git::tests::destructive_guard` — all four tests pass
- `cargo test` — full suite passes
- `cargo clippy --all-targets -- -D warnings` — clean

## Key constraint
- Do NOT use `std::env::set_current_dir` anywhere in the fix — that's the whole point.
- The function stays `#[cfg(test)]`-gated in the `run_git` call site — that doesn't change.
