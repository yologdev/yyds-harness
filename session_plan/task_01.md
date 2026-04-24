Title: Fix home directory hang — cap walk_directory and expand ignore list
Files: src/commands_search.rs
Issue: #333

## Problem

When yoyo is run from `~` (not a git repo), `list_project_files()` falls through to `walk_directory(".", 8)` which recursively walks the entire home directory. This hangs for minutes because:

1. No file count cap — walks millions of files in `go/pkg/mod/`, `.cache/`, etc.
2. Only skips `.`-prefixed, `node_modules`, and `target` — misses many common cache/dependency dirs.
3. Max depth of 8 is too deep for a non-project fallback.

## Fix

In `walk_directory_inner()` (around line 168 in `src/commands_search.rs`):

1. **Add a file count cap.** Change `files: &mut Vec<String>` pattern to bail once `files.len()` reaches a cap (e.g., 10,000). Pass through a shared counter or check `files.len()` before pushing. Return early once the cap is hit.

2. **Expand the ignore list.** Add these common cache/dependency directories to the skip check alongside `node_modules` and `target`:
   - `go` (catches `go/pkg/mod/`)
   - `.cache`
   - `.local`
   - `.cargo`
   - `.npm`
   - `.nvm`
   - `.rustup`
   - `Library` (macOS)
   - `__pycache__`
   - `venv`
   - `.venv`
   - `vendor`
   - `.gradle`
   - `.m2`
   - `dist`
   - `build`
   - `.tox`
   - `.mypy_cache`
   - `.pytest_cache`
   - `coverage`
   - `.next`
   - `.nuxt`
   - `bower_components`

   Note: hidden dirs (starting with `.`) are already skipped. So `.cache`, `.local`, `.cargo`, `.npm`, `.nvm`, `.rustup`, `.venv`, `.gradle`, `.m2`, `.tox`, `.mypy_cache`, `.pytest_cache`, `.next`, `.nuxt` are already caught by `name.starts_with('.')`. Only need to add the non-hidden ones: `go`, `Library`, `__pycache__`, `venv`, `vendor`, `dist`, `build`, `coverage`, `bower_components`.

3. **Reduce fallback max depth.** In `list_project_files()` line ~158, change `walk_directory(".", 8)` to `walk_directory(".", 4)`. Depth 8 is excessive for a non-git fallback — 4 levels is plenty to find files in a reasonable project.

4. **Update existing tests** and add a new test for the file count cap behavior.

## Verification

- `cargo build && cargo test`
- Existing `walk_directory_*` tests must still pass
- New test: create a dir structure exceeding the cap, verify `walk_directory` returns at most the cap
