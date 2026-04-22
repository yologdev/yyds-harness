Title: Clean up stale artifact and harden /extended with --budget flag
Files: dispatch_command_output.rs (delete), src/repl.rs
Issue: #278

## What to do

### Part 1: Remove stale file
Delete `dispatch_command_output.rs` from the repo root. It's a 576-line dead artifact — not compiled, not referenced by any source file or Cargo.toml. Just `git rm dispatch_command_output.rs`.

### Part 2: Add --budget (time limit) to /extended

Issue #278 specifically asks for `--budget` or `--time` limit on extended tasks. The current `/extended` command only supports `--turns N`. Add a `--budget N` flag that sets a wall-clock time limit in minutes.

Implementation in `src/repl.rs`:

1. In `parse_extended_args`, add parsing for `--budget N` (minutes). Return a tuple `(String, usize, Option<Duration>)` instead of `(String, usize)`.

2. In `handle_extended`, if a budget is provided, use `tokio::time::timeout` around the `run_prompt_auto_retry` call. On timeout, print a clear message:
   ```
   🐙 Extended mode — time budget exhausted (N min)
   ```
   Then fall through to the normal summary (files changed, etc).

3. Update the usage help text to show:
   ```
   Usage: /extended <task description> [--turns N] [--budget N]
   
   Examples:
     /extended add error handling to the parser module
     /extended refactor the auth system --turns 30
     /extended rebuild the test suite --budget 15
   ```

4. Update existing tests for `parse_extended_args` to handle the new return type, and add tests:
   - `test_parse_extended_budget` — `"/extended do stuff --budget 10"` → budget = Some(10 min)
   - `test_parse_extended_turns_and_budget` — both flags together
   - `test_parse_extended_no_budget` — no flag → budget = None

This is a concrete, user-requested enhancement that moves /extended closer to what Issue #278 envisions. The separate evaluation agent part is a larger future task — this lands the time-budget piece first.

Update help.rs if /extended has a help entry there.
