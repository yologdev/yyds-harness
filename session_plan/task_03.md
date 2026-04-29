Title: Two-phase watch: separate lint and test phases with distinct fix prompts
Files: src/watch.rs, src/commands_dev.rs
Issue: none

## Problem

When `/watch all` is active, the watch system runs `cargo clippy && cargo test`
(or equivalent) as a single shell command. When it fails:
- If clippy fails, the fix prompt gets clippy output — good.
- If clippy passes but tests fail, the fix prompt gets test output — good.
- But the fix prompt doesn't *know* which phase failed, so it can't tailor its
  strategy (lint fixes are usually mechanical; test fixes require understanding).
- More importantly: after fixing a lint error, the system re-runs the full
  `clippy && test` command instead of just re-running clippy to verify the lint
  fix before moving to tests.

Aider's signature feature is running lint and test as separate phases:
lint → fix lint → re-lint → test → fix test → re-test. This is more efficient
because lint fixes are fast and targeted, while test fixes need more context.

## What to do

1. **Add a `set_watch_commands` function** (plural) alongside the existing
   `set_watch_command` (singular) in `watch.rs`. This stores a `Vec<String>`
   of commands to run in sequence, where each command is its own phase with
   its own fix loop. The singular version remains for backward compatibility
   and stores a single-element vec.

2. **Add `get_watch_commands` function** that returns the vec. The existing
   `get_watch_command` returns the first command or the joined string for display.

3. **Modify `run_watch_after_prompt`** (or add a new `run_watch_phases`) to:
   - Iterate through the watch commands in order
   - For each command: run it, if it fails enter the fix loop (up to
     MAX_WATCH_FIX_ATTEMPTS), if fix loop exhausts → stop and report
   - Only proceed to the next command if the current one passes
   - This way lint gets fixed before tests even run

4. **Update `/watch all` in `commands_dev.rs`** to call `set_watch_commands`
   with a two-element vec `["cargo clippy ...", "cargo test"]` instead of
   joining them with `&&`.

5. **Update the fix prompt** in `build_watch_fix_prompt` to include a hint
   about what kind of command failed (lint vs test), so the agent can choose
   the right fix strategy.

6. **Add tests** for the multi-phase watch loop:
   - Test that `set_watch_commands` + `get_watch_commands` round-trips
   - Test that single-command mode still works
   - Test that the fix prompt includes the command type hint

7. Run `cargo build && cargo test && cargo clippy --all-targets -- -D warnings`.

## Important constraints
- Don't break existing `/watch <command>` behavior — single commands still work
- Don't break `auto_detect_watch_command` — it returns a single string, and
  that's fine for auto-detection (it can be split internally)
- Keep `get_watch_command` (singular) working for backward compat — it returns
  the first command or joins them for display purposes
- The fix loop per phase should use the same MAX_WATCH_FIX_ATTEMPTS constant

## Verification
- `cargo test` passes
- `/watch all` now runs lint and test as separate phases
- `/watch cargo test` still works as before (single-phase)
- Fix prompts mention whether the failure is from lint or test
