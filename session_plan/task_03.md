Title: Extract update-checking logic from cli.rs into src/update.rs
Files: src/cli.rs, src/main.rs
Issue: none

## What

cli.rs is 4,232 lines — the largest source file. Extract the update-checking and version-comparison logic into a new `src/update.rs` module. This is a targeted extraction of a self-contained concern (~80-120 lines including tests).

## What to extract

1. **`version_is_newer(current, latest)`** function — semver comparison logic
2. **`check_for_update()`** function — fetches latest version from crates.io, compares, prints update message
3. Their associated tests (search for `test_version_is_newer` and any `test_check_for_update` tests)

These functions have no dependencies on cli.rs internals — they use `VERSION` (which is a constant that can be passed as parameter or re-exported) and standard library types.

## How

1. Create `src/update.rs` with the extracted functions
2. `version_is_newer` stays `pub` with same signature
3. `check_for_update` needs the current version string — either pass `VERSION` as a parameter or import `crate::cli::VERSION`. Prefer passing as parameter for testability: `pub fn check_for_update(current_version: &str)`
4. Add `pub mod update;` in `main.rs`
5. In `cli.rs`, replace the function bodies with calls to `crate::update::*`, or just remove them and update call sites to use `crate::update::check_for_update(VERSION)` directly
6. Move tests along with the functions
7. Run `cargo build && cargo test && cargo clippy --all-targets -- -D warnings`

## What NOT to do

- Don't extract config file parsing (that's a bigger task)
- Don't extract argument parsing (core cli.rs responsibility)
- Don't touch any other cli.rs functions — just the update-checking pair
- Keep it small — this is a ~100-line extraction, not a cli.rs overhaul

## Verification

- `cargo build` passes
- `cargo test` passes
- `cargo clippy --all-targets -- -D warnings` passes
- `check_for_update` still works (called during startup)
- `version_is_newer` tests still pass from new location
