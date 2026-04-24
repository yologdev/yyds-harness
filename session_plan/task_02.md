Title: Bake DAY_COUNT at compile time — fix banner for non-self-hosted users
Files: build.rs, src/cli.rs
Issue: #331

## Problem

The REPL banner reads `DAY_COUNT` from disk at runtime:

```rust
let day_suffix = std::fs::read_to_string("DAY_COUNT")
    .ok()
    .and_then(|s| s.trim().parse::<u32>().ok())
    .map(|d| format!(" — Day {d}"))
    .unwrap_or_default();
```

This only works when running from yoyo's own repo. External users see no day. The file is a repo artifact, not something distributed with releases.

## Fix

### 1. In `build.rs`: Read DAY_COUNT at compile time

Add a new block (similar to GIT_HASH and BUILD_DATE) that reads the `DAY_COUNT` file if it exists and sets a compile-time env var:

```rust
// Expose evolution day count at compile time (only present in yoyo's own repo)
if std::env::var("DAY_COUNT").is_err() {
    if let Ok(content) = std::fs::read_to_string("DAY_COUNT") {
        if let Ok(day) = content.trim().parse::<u32>() {
            println!("cargo:rustc-env=DAY_COUNT={day}");
        }
    }
}
```

Also add `cargo:rerun-if-changed=DAY_COUNT` so the build re-reads when the day changes.

### 2. In `src/cli.rs`: Use compile-time value in `print_banner()`

Replace the runtime file read with:

```rust
pub fn print_banner() {
    let day_str = option_env!("DAY_COUNT").unwrap_or("");
    let day_suffix = if day_str.is_empty() {
        String::new()
    } else {
        format!(" — Day {day_str}")
    };
    println!(
        "\n{BOLD}{CYAN}  yoyo{RESET} v{VERSION}{day_suffix} {DIM}— a coding agent growing up in public{RESET}"
    );
    println!("{DIM}  Type /help for commands, /quit to exit{RESET}\n");
}
```

This way:
- In yoyo's own repo builds: DAY_COUNT is baked in at compile time
- In external user builds: `option_env!("DAY_COUNT")` returns `None`, day is omitted cleanly
- In release builds: the CI can set `DAY_COUNT` env var to override

### 3. Update the banner test

The test `test_print_banner_with_day_count` creates a DAY_COUNT file and changes to that directory. Since we're now reading at compile time, the test needs to change. The compile-time value is whatever DAY_COUNT was when `cargo test` ran. Since we're in the repo, it should have the current day. Update the test to verify the banner function doesn't panic (which it already does) — the compile-time embedding is implicitly tested by the build succeeding.

Remove or simplify the test that creates a temp DAY_COUNT file since it's no longer relevant (the file is read at build time, not runtime).

## Verification

- `cargo build && cargo test`
- `cargo run -- --version` should still work
- `print_banner()` should show the day when built from this repo
- When DAY_COUNT file doesn't exist (external build), `option_env!` returns None gracefully
