Title: Add output tokens/sec to usage line and /profile display
Files: src/format/mod.rs, src/commands_info.rs
Issue: none

## What

After every agent turn, developers see a usage line like:
```
↳ 3.2s · 1234→567 tokens · $0.012
```

Add output tokens/sec so developers can monitor LLM generation speed:
```
↳ 3.2s · 1234→567 tokens (177 tok/s) · $0.012
```

This is a small but highly visible UX improvement — it fires after every single turn.

## Changes

### 1. `src/format/mod.rs` — `format_usage_line`

In the non-verbose branch, calculate and display tokens/sec:
- `let tok_per_sec = if elapsed.as_secs_f64() > 0.1 { Some((usage.output as f64 / elapsed.as_secs_f64()) as u32) } else { None };`
- Only show if elapsed > 0.1s (avoid division by zero or misleading infinity for cached responses)
- Format: `(177 tok/s)` appended after the token counts
- In verbose mode, also add a speed line: `speed: 177 tok/s`

Also add tests for the new format:
- Test that tok/s appears when elapsed > 0.1s
- Test that tok/s is omitted when elapsed is tiny (< 0.1s)
- Test that the calculation is correct (e.g., 100 output tokens in 2.0s = 50 tok/s)

### 2. `src/commands_info.rs` — `handle_profile`

Add a "Speed" row to the profile box showing average output tokens/sec for the session:
- Calculate: `session_total.output as f64 / elapsed.as_secs_f64()`
- Add after "Tokens" row: `("Speed", &speed_str, speed_str.clone())`
- Only show if elapsed > 1.0s (profile is meaningless for sub-second sessions)
- Format: `~177 tok/s`

## Rules
- Don't modify the print_usage function signature — it already passes elapsed
- Keep the format compact — tok/s should fit naturally in the existing line
- Add tests for format_usage_line covering the new tok/s display
- Run `cargo test` to verify
