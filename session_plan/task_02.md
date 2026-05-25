Title: Comprehensive test coverage for help_data.rs
Files: src/help_data.rs
Issue: none

## What

Add a comprehensive test module to `src/help_data.rs`. This file is 1,312 lines with zero tests — the only file over 200 lines with no test coverage. It contains `command_help()` and `command_short_description()` which are critical for user-facing discoverability.

## Why

The assessment flagged this as a quality gap. While `src/help.rs` has a few tests that call into `help_data.rs` functions, the data file itself has no self-contained tests. Property-style tests can catch data integrity issues (missing entries, stale descriptions, inconsistencies) that point-tests in `help.rs` miss.

## Implementation

Add a `#[cfg(test)] mod tests` block at the bottom of `src/help_data.rs` with these tests:

### Completeness tests

1. **`test_every_known_command_has_help`** — Import `KNOWN_COMMANDS` from `crate::commands`. For every command in `KNOWN_COMMANDS` (stripping the leading `/`), verify `command_help(name)` returns `Some`. Skip `/exit` (alias for `/quit`).

2. **`test_every_known_command_has_short_description`** — For every command in `KNOWN_COMMANDS`, verify `command_short_description(name)` returns `Some`. Skip `/exit`.

3. **`test_help_entries_match_known_commands`** — Verify there are no "orphan" help entries that don't correspond to any known command. Call `command_help()` with a list of all KNOWN_COMMANDS and verify they all return content. Then call with a fake command like `"zzz_nonexistent"` and verify it returns `None`.

### Content quality tests

4. **`test_short_descriptions_are_actually_short`** — Every short description should be ≤ 80 chars (one line). This catches accidentally long descriptions.

5. **`test_help_entries_are_non_empty`** — Every help entry returned by `command_help()` should have at least 20 characters (enough for a meaningful help text).

6. **`test_help_does_not_contain_leading_slash`** — `command_help("add")` and `command_help("/add")` should return the same result (the function should strip leading slashes).

### Edge case tests

7. **`test_command_help_returns_none_for_empty`** — `command_help("")` returns `None`.

8. **`test_command_short_description_returns_none_for_unknown`** — `command_short_description("zzz_nonexistent")` returns `None`.

9. **`test_no_duplicate_short_descriptions`** — All short descriptions should be unique (no two commands with identical descriptions, which would indicate copy-paste errors).

### Import note

You'll need to import `KNOWN_COMMANDS` from `crate::commands`. Check the exact import path — it should be `use crate::commands::KNOWN_COMMANDS;`.

## Verification

- `cargo test help_data` — all new tests pass
- `cargo clippy --all-targets -- -D warnings` — clean
- The tests serve as a regression guard: if someone adds a new command to KNOWN_COMMANDS but forgets to add help text, these tests will catch it.
