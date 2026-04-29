Title: Extract lint/test handlers from commands_dev.rs into commands_lint.rs
Files: src/commands_lint.rs (new), src/commands_dev.rs, src/dispatch.rs
Issue: none

## Problem

`commands_dev.rs` is 2,532 lines and contains seven distinct command families:
/doctor, /health, /fix, /test, /lint (with /lint fix, /lint unsafe), /watch, /tree.
The lint and test handlers form a cohesive group (~500 lines) that can be extracted
into their own module, following the same pattern used for commands_run.rs (Day 59),
commands_bg.rs, commands_spawn.rs, etc.

## What to extract into `commands_lint.rs`

From `commands_dev.rs`, move these items:
- `fn test_command_for_project` — detects test command for the project type
- `fn handle_test` — the /test handler
- `enum LintStrictness` — lint strictness levels
- `const LINT_SUBCOMMANDS` — tab-completion list for /lint subcommands
- `fn lint_command_for_project` — detects lint command for the project type
- `fn handle_lint` — the /lint handler
- `fn build_lint_fix_prompt` — builds the prompt for AI-powered lint fixing
- `fn handle_lint_fix` — the /lint fix handler (async)
- `struct UnsafeOccurrence` + `enum UnsafeKind` + their impls
- `fn scan_for_unsafe` — scans for unsafe code
- `fn has_unsafe_code_attribute` — checks for unsafe code attributes
- `fn handle_lint_unsafe` — the /lint unsafe handler
- All associated tests for the above

Keep in `commands_dev.rs`: /doctor, /health, /fix, /watch, /tree, and their helpers.

## Steps

1. Create `src/commands_lint.rs` with the extracted items.
2. Add `pub mod commands_lint;` to `main.rs`.
3. Update `commands_dev.rs` to remove the extracted items.
4. Update `dispatch.rs` to import from `commands_lint` instead of `commands_dev`
   for lint/test dispatching. Check all call sites with:
   `grep -rn "commands_dev::handle_test\|commands_dev::handle_lint\|commands_dev::lint_command\|commands_dev::test_command\|commands_dev::LintStrictness\|commands_dev::LINT_SUBCOMMANDS\|commands_dev::build_lint_fix_prompt\|commands_dev::UnsafeOccurrence\|commands_dev::scan_for_unsafe" src/`
5. Also check `commands_dev.rs` internal references — `detect_watch_all_command`
   calls `test_command_for_project` and `lint_command_for_project`. These will
   need to be imported from `commands_lint` after the move.
6. Update the `detect_project_type` references — the `ProjectType` enum and
   `detect_project_type` function are used by both lint and watch handlers. If
   `ProjectType` is defined in `commands_dev.rs`, leave it there and import it
   in `commands_lint.rs` (don't move types used by both modules).
7. Run `cargo build && cargo test && cargo clippy --all-targets -- -D warnings`.

## Verification
- `commands_dev.rs` drops by ~500 lines
- `commands_lint.rs` is a self-contained ~500 line module
- All existing tests pass in their new location
- No behavior change for users
