Title: Show session resume hint at startup when auto-saved session exists
Files: src/banner.rs, src/main.rs
Issue: none

## What

Claude Code shows "Resume previous conversation?" when a saved session exists. We auto-save
sessions to `.yoyo/last-session.json` on exit, but if the user starts yoyo without `--continue`,
they get no indication that a previous session is available. This is a UX gap — users may not
know they can resume.

## How

1. In `src/banner.rs`, add a function `session_resume_hint() -> Option<String>` that:
   - Checks if `.yoyo/last-session.json` exists
   - If it does, reads its modification time
   - Formats a hint like: `"  💬 Previous session available (2h ago) — use --continue to resume"`
   - Returns `None` if the file doesn't exist or is older than 7 days (stale sessions aren't useful)
   - Uses `DIM` color for subtlety

2. In `src/main.rs`, in the REPL startup path (after `print_banner` but before `run_repl`),
   if `continue_session` is false, call `session_resume_hint()` and print it if Some.

3. Add tests in `banner.rs`:
   - `session_resume_hint` returns None when no file exists
   - `session_resume_hint` returns Some when file exists (use temp dir with a test file)
   - `session_resume_hint` returns None for files older than 7 days (if testable, otherwise skip)

## Details

- Use `std::fs::metadata` to get modification time
- Use `SystemTime::elapsed()` to compute age
- Format relative time: "just now" (<1min), "Xm ago" (<1h), "Xh ago" (<24h), "Xd ago" (<7d)
- The hint should use the `AUTO_SAVE_SESSION_PATH` constant from `cli_config.rs`

## Scope

Only `src/banner.rs` (new function + tests) and `src/main.rs` (1-2 lines to call it).
