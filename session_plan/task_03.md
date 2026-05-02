Title: Extract /plan command into commands_plan.rs
Files: src/commands_plan.rs (new), src/commands_project.rs, src/commands.rs
Issue: none

`commands_project.rs` (2,028 lines) bundles four distinct concerns: `/context`, `/init`, `/docs`, `/plan`. This task extracts the `/plan` concern into its own `commands_plan.rs`, continuing the consolidation arc from Days 58-63.

**What to move to `src/commands_plan.rs`:**

Production code:
- `PLAN_MODE` static `AtomicBool` (line ~25)
- `set_plan_mode()` function (line ~27)
- `is_plan_mode()` function (line ~32)
- `PLAN_MODE_PROMPT` constant (line ~37)
- `PLAN_SUBCOMMANDS` constant (line ~734)
- `parse_plan_task()` function (line ~738)
- `build_plan_prompt()` function (line ~753)
- `handle_plan()` async function (line ~783)

Tests (move to `#[cfg(test)] mod tests` in the new file):
- `test_parse_plan_task_extracts_task`
- `test_parse_plan_task_empty_returns_none`
- `test_build_plan_prompt_structure`
- `test_plan_mode_toggle`
- `test_parse_plan_task_skips_mode_keywords`
- `test_plan_mode_prompt_content`
- `test_plan_subcommands`
- `test_plan_in_known_commands` (may need to stay in commands_project.rs if it tests cross-module things)
- `test_plan_in_help_text` (same — check if it references plan-specific logic)

**What to update in `src/commands.rs`:**
- Change the re-export of `handle_plan`, `is_plan_mode`, `PLAN_MODE_PROMPT` from `crate::commands_project` to `crate::commands_plan`
- Add `PLAN_SUBCOMMANDS` re-export from `crate::commands_plan` if needed for completions

**What to update in `src/commands_project.rs`:**
- Remove the moved functions, constants, and tests
- Remove any imports that were only used by the plan code (check `use crate::prompt::*` — it's used by handle_plan's `run_prompt` call)
- Update the module doc comment to remove `/plan` mention

**Dependencies the new file needs:**
- `use crate::format::*;` for `GREEN`, `DIM`, `RESET`
- `use crate::prompt::{run_prompt};` for the prompt execution
- `use crate::commands_session::auto_compact_if_needed;` for auto-compaction
- `use std::sync::atomic::{AtomicBool, Ordering};`
- `use yoagent::agent::Agent;`
- `use yoagent::*;` for `Usage`

**Don't forget:**
- Add `mod commands_plan;` in `main.rs`
- Run `cargo build && cargo test` to verify zero behavior change
- The `PLAN_SUBCOMMANDS` completion in `commands.rs` line ~267 may need the import path updated

**Sizing:** 3 files touched (new file + 2 edits). ~134 lines of production code + ~80 lines of tests to move. Mechanical extraction, ~15 minutes.
