Title: Exit summary — show what changed during the session
Files: src/repl.rs, src/session.rs, src/format/mod.rs
Issue: none

## Context

When a developer finishes a yoyo session (Ctrl+D or `/quit`), the tool exits silently after auto-saving. There's no summary of what happened — how many files were changed, what commands were run, how many tokens were used. Claude Code shows a brief summary on exit. For a developer who just spent 30 minutes in a session, a quick recap is valuable — it confirms what happened and catches anything unexpected.

The `SessionChanges` struct in `session.rs` already tracks file changes during the session. The cost/token information is available from the agent. This task connects existing infrastructure to the exit path.

## What to do

1. **Add an `exit_summary` function** (in `format/mod.rs` or `session.rs`) that takes:
   - The agent's messages (to compute total tokens/cost)
   - The `SessionChanges` (to show files modified/created)
   - Session start time (to show duration)
   
   And returns a formatted string like:
   ```
   ─── session summary ────────────────────
   Duration: 12m 34s · Tokens: 45.2K in / 12.1K out · Cost: ~$0.23
   Files changed: 3 (src/main.rs, src/lib.rs +new, tests/test.rs)
   ```

2. **Call the exit summary in `run_repl`** just before the existing `auto_save_on_exit` call (around line 978). Only print it if the session had at least 1 turn (don't print a summary for empty sessions where the user immediately quit).

3. **Track session start time.** Add a `let session_start = std::time::Instant::now();` at the top of `run_repl` and pass it to the summary function.

4. **Keep it minimal.** The summary should be 2-3 lines max. No fancy boxes or heavy formatting — just a clean divider line and the key stats. Use `DIM` color so it doesn't compete with the conversation content.

5. **Write tests** for the formatting function — test with 0 files, 1 file, multiple files, with and without new files.

## What NOT to do

- Don't add any interactive prompts on exit — this is informational only
- Don't change the auto-save behavior
- Don't import heavy dependencies — use existing format utilities
- Don't show the summary in piped/non-interactive mode (only REPL)

## Verification

- `cargo build` passes
- `cargo test` passes (including new formatting tests)
- `cargo clippy --all-targets -- -D warnings` passes
