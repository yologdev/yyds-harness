Title: Auto-enable watch mode in piped/single-prompt mode with auto-fix feedback
Files: src/prompt.rs, src/repl.rs
Issue: none

## Problem

yoyo already has a complete auto lint-fix-test loop: `/watch` sets a command, the REPL
runs it after agent edits, and failures are fed back to the agent for up to 3 auto-fix
attempts. There's even `auto_watch` config support.

However, this only works in REPL mode. When yoyo is used in single-prompt mode
(`yoyo "fix the bug in main.rs"`) or piped mode, the watch loop never fires because
`run_repl()` is never called — the prompt goes through `run_prompt_auto_retry()` or
`run_prompt_auto_retry_with_content()` directly.

The single biggest Aider gap identified in the assessment is that "after every AI edit →
run linter → if errors, feed back → auto-fix → repeat" isn't integrated outside the REPL.

## Implementation

1. In `src/prompt.rs`, add a function `run_watch_after_prompt()` that:
   - Checks if a watch command is active (`get_watch_command()`)
   - If so, runs it via `run_watch_command()`
   - If it fails, builds a fix prompt with `build_watch_fix_prompt()` and calls
     `run_prompt_auto_retry_with_content()` with the fix prompt
   - Loops up to `MAX_WATCH_FIX_ATTEMPTS` times
   - Returns the final (success, output) state

2. In `src/repl.rs`, in the `handle_quick()` and `handle_extended()` functions (which
   handle single-prompt and piped mode), after the prompt completes, call
   `run_watch_after_prompt()` if the agent modified files. This mirrors what the REPL
   loop already does at line ~656.

3. Also in `src/repl.rs`, in the piped-mode path (the `!is_interactive` branch in
   `run_repl`), check if auto_watch is enabled and auto-detect a watch command, same
   as the REPL path does at line 374. This ensures piped-mode users get the same
   auto-watch behavior.

4. Add tests for `run_watch_after_prompt`:
   - Test with no watch command set → returns immediately
   - Test with a passing watch command → returns (true, output)
   - Test with a failing command → attempts fix (mock the prompt part, just test the loop structure)

## Verification

- `cargo build && cargo test`
- The existing watch-related tests should still pass
- Manual test: `yoyo --watch "cargo test" "add a greeting function to main.rs"` should
  run tests after the agent edits and auto-fix if they fail

## Note

This task focuses on making the existing watch infrastructure available in non-REPL modes.
It does NOT change the watch loop logic itself — that already works well in the REPL.
Keep the implementation simple: extract the watch-after-edit logic from the REPL loop into
a reusable function, then call it from the non-REPL paths.
