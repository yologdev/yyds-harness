Title: Add native desktop notifications on long completions
Files: src/format/mod.rs
Issue: none

## What

yoyo currently rings the terminal bell (BEL character `\x07`) when a prompt takes ≥3 seconds.
This only works if the terminal is focused and the user's terminal emulator supports it.
Desktop notifications (macOS Notification Center, Linux notify-send, Windows toast) are
a better signal for long-running tasks — the user can switch to another window and still
know when yoyo is done. Aider has this; yoyo doesn't.

## Implementation

Add a `send_desktop_notification()` function in `src/format/mod.rs` next to the existing
`maybe_ring_bell()` function. Then call it from `maybe_ring_bell()` when the elapsed time
exceeds a higher threshold (e.g., 10 seconds — the bell fires at 3s, but desktop notifications
should be reserved for genuinely long waits to avoid notification fatigue).

The implementation should be pure shell commands (no external Rust crate dependency):
- **macOS**: `osascript -e 'display notification "..." with title "yoyo"'`
- **Linux**: `notify-send "yoyo" "..."`
- **Windows**: PowerShell toast notification via `powershell -Command "..."`

Use `std::process::Command` to fire-and-forget (spawn, don't wait). If the notification
command fails (not installed, etc.), silently ignore — it's a best-effort enhancement.

Add a `--no-notify` CLI flag that disables desktop notifications (similar to `--no-bell`).
Also respect a `YOYO_NO_NOTIFY` environment variable. Use the same OnceLock pattern as
`bell_enabled()` / `disable_bell()`.

Modify `maybe_ring_bell()` to also call `send_desktop_notification()` when elapsed ≥ 10s
and notifications are enabled.

### Tests

- Test that `send_desktop_notification` doesn't panic (call it, ignore the result)
- Test the disable/enable flag logic (similar to existing bell tests)
- Test that the notification message is reasonable (contains "yoyo" and a duration)

### CLI flag wiring

In `src/main.rs` where `--no-bell` is handled (around line 450), add `--no-notify` handling
that calls `disable_notify()`. Also check `YOYO_NO_NOTIFY` env var.

In `src/help.rs`, add `--no-notify` to the CLI help text near `--no-bell`.

## Files touched

- `src/format/mod.rs` — notification logic, enable/disable state
- `src/main.rs` — `--no-notify` flag parsing (1 line addition near `--no-bell`)
- `src/help.rs` — help text update (1 line addition)

Wait — that's 3 files. Let me consolidate: the main.rs and help.rs changes are each 1-2 lines.
The core implementation is in format/mod.rs. This stays within the 3-file limit.

## Why

- Listed in assessment as "Desktop notifications (Aider) — simple feature, not yet implemented. Low effort, nice UX"
- Competitive gap vs Aider
- Natural extension of existing bell mechanism
- No external dependencies needed
- First-contact quality improvement — users notice when tools respect their attention

## Verification

- `cargo build && cargo test`
- Manual test on macOS/Linux: run a prompt, verify notification appears after 10s
