Title: Extract /skill handling from commands_project.rs into commands_skill.rs
Files: src/commands_project.rs, src/commands_skill.rs (new), src/commands.rs
Issue: none

`commands_project.rs` is 2,736 lines and bundles five distinct concerns: /todo, /context, /init, /plan, AND /skill. The /skill subsystem is growing (list, show, path, install) and deserves its own module.

## What to extract

All skill-related functions from `commands_project.rs` into a new `src/commands_skill.rs`:

1. `handle_skill()` — the top-level dispatcher (public)
2. `skill_list()` — list loaded skills
3. `skill_show()` — show a single skill's details
4. `skill_path()` — show the skill directory path
5. `skill_install()` — install from local path
6. `skill_install_to()` — the testable inner function
7. `default_skill_install_dir()` — helper for install dir
8. `SKILL_SUBCOMMANDS` constant
9. All skill-related tests (grep for `skill` in the test module)

## How to do it

1. Create `src/commands_skill.rs` with the moved functions and their imports
2. Remove the skill functions from `commands_project.rs`
3. Add `mod commands_skill;` to `main.rs`
4. Update `src/commands.rs` re-export: change the `commands_project` re-export line to remove `handle_skill`, and add a new `pub use crate::commands_skill::handle_skill;` line
5. Update `src/commands.rs` completion for `/skill` — change `crate::commands_project::SKILL_SUBCOMMANDS` to `crate::commands_skill::SKILL_SUBCOMMANDS`
6. Check `dispatch.rs` for any direct references to `commands_project::handle_skill` — update to `commands_skill` or leave through the `commands::` re-export
7. Run `cargo build && cargo test && cargo clippy --all-targets -- -D warnings`

## Important details
- `commands_project.rs` currently re-exports refactor functions via `pub use crate::commands_refactor::...`. Don't touch those.
- `commands.rs` line 451-454 re-exports from `commands_project` — split `handle_skill` out to its own line from `commands_skill`
- `commands.rs` line ~260 references `crate::commands_project::SKILL_SUBCOMMANDS` for completions — update this path
- `dispatch.rs` may reference `commands_project::handle_skill` or go through `commands::handle_skill` — check and update

## Rules
- Zero behavior changes — same functions, same signatures, same test results
- Don't rename anything, just move
- Keep the same test structure
