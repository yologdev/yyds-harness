Title: Consolidate inline watch-fix loop in repl.rs to use run_watch_after_prompt
Files: src/repl.rs, src/watch.rs
Issue: none

## Problem

The main REPL loop in `repl.rs` (lines ~788-844) contains ~55 lines of inline watch-fix
logic that duplicates `run_watch_after_prompt()` in `watch.rs`. Both implementations:
- Check if a watch command is set
- Run the watch command
- Show truncated output on failure
- Enter a multi-attempt auto-fix loop (up to MAX_WATCH_FIX_ATTEMPTS)
- Call `build_watch_fix_prompt` and `run_prompt_auto_retry` in the loop
- Check `session_budget_exhausted(30)` before each attempt

The only difference is the inline version also sets `last_error` from each fix outcome
and checks `files_modified` before running. The `run_watch_after_prompt` function
doesn't return enough info for the caller to update `last_error`.

## What to do

1. **Modify `run_watch_after_prompt` in `watch.rs`** to return a richer result type
   instead of just `bool`. Add a small struct or enum that carries:
   - Whether the watch passed (bool)
   - The last tool error from fix attempts (Option<String>)
   
   Something like:
   ```rust
   pub struct WatchResult {
       pub passed: bool,
       pub last_tool_error: Option<String>,
   }
   ```

2. **Add a `files_modified` guard** — `run_watch_after_prompt` already returns early
   if no watch command is set. Add an optional `files_modified: bool` parameter or
   have the caller check before calling (simpler).

3. **Replace the inline watch loop in `repl.rs`** (lines ~788-844) with a call to
   `run_watch_after_prompt`, using the new return type to update `last_error`.

4. **Add tests** for the new `WatchResult` type and ensure existing watch tests still pass.

5. Run `cargo build && cargo test && cargo clippy --all-targets -- -D warnings`.

## Verification
- `cargo test` passes (all existing watch tests still green)
- The inline code in repl.rs is replaced with a ~5-line call
- No behavior change for the user
