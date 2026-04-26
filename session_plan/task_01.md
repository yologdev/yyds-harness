Title: Suppress spinner and progress ANSI escape sequences when stderr is not a TTY
Files: src/format/tools.rs, src/format/mod.rs
Issue: none

## Problem

The assessment found that spinner artifacts (`⠋ thinking...[K[K`) leak into output when
stderr is not a terminal (e.g., captured output, piped through `2>&1 | less`, CI logs).
All spinner and progress output uses `eprint!` (correct — stderr), but the ANSI escape
sequences (`\x1b[K`, `\r`) are not suppressed when stderr isn't a TTY.

## Implementation

1. In `src/format/tools.rs`, add a check at the top of the `Spinner::start()` method:
   if `!std::io::stderr().is_terminal()`, skip starting the spinner thread entirely
   (set `self.running` to false or return immediately). This prevents all spinner
   escape sequences from being emitted.

2. Similarly, in `ToolProgressTimer::start()`, check `std::io::stderr().is_terminal()`
   before spawning the progress thread. If not a TTY, skip the progress display.

3. The `stop()` methods for both `Spinner` and `ToolProgressTimer` already emit
   `eprint!("\r\x1b[K")` to clear the line. These should also be gated on
   `std::io::stderr().is_terminal()` — no need to clear a line that was never drawn.

4. Add a helper function `fn stderr_is_terminal() -> bool` in `format/mod.rs` (or
   `format/tools.rs`) that caches the result of `std::io::stderr().is_terminal()` using
   a `std::sync::OnceLock<bool>` to avoid repeated syscalls. Import `std::io::IsTerminal`.

5. Add tests:
   - Test that `stderr_is_terminal()` returns a bool (basic smoke test)
   - Test that `Spinner` can be created and stopped without panicking (doesn't need
     terminal verification — just structural soundness)

## Verification

- `cargo build && cargo test`
- Manual: `echo "hello" | cargo run 2>&1 | cat` should show no `⠋ thinking...` or `[K` artifacts
- The existing spinner/progress tests should still pass
