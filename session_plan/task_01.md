Title: Fix auto-watch message leak in --print mode
Files: src/main.rs
Issue: none

## Description

The assessment identified that `--print` mode (designed for machine-readable output with zero chrome) still leaks the auto-watch announcement to stderr:

```
👀 Auto-watch: `cargo clippy ...` (disable with auto_watch = false)
```

This happens in TWO locations:
1. `run_single_prompt()` around line 217-220 — the auto-watch block runs unconditionally even when `print_mode` is true
2. `run_piped_mode()` around line 421-424 — same issue

The model announcement is correctly guarded by `if !print_mode` (lines 202, 413), but the auto-watch block 15 lines below is NOT wrapped in the same guard.

## Fix

Wrap both auto-watch blocks in `if !print_mode { ... }`:

Location 1 (~line 217):
```rust
if !print_mode {
    if get_watch_command().is_none() && agent_config.auto_watch {
        if let Some(cmd) = watch::auto_detect_watch_command() {
            set_watch_command(&cmd);
            eprintln!("{DIM}  👀 Auto-watch: `{cmd}` (disable with auto_watch = false){RESET}");
        }
    }
}
```

Location 2 (~line 421):
Same pattern — wrap the auto-watch block in `if !print_mode { ... }`.

Note: The watch command should still be SET even in print mode (the underlying behavior is fine), just the eprintln announcement should be suppressed. Actually, looking more carefully — in `--print` mode we probably don't want auto-watch at all since watch mode runs after prompts in interactive/piped modes. But the minimal fix is: suppress the announcement. If the watch behavior itself is also problematic in --print, that's a separate issue.

## Verification

```bash
cargo build && cargo test
# Manual: echo "hello" | cargo run -- --print  (should have no 👀 line on stderr)
```
