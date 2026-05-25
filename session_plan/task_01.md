Title: Make auto_commit configurable from .yoyo.toml
Files: src/config.rs, src/cli.rs
Issue: none

## What

Add `auto_commit` as a settable config key in `.yoyo.toml`, following the existing pattern used by `auto_watch`. Currently, `--auto-commit` only works as a CLI flag, meaning users who always want auto-commit must type it every time. Per-project config support lets users set it once in `.yoyo.toml`.

## Why

This closes a workflow gap against Aider, which auto-commits by default. Users who want auto-commit behavior shouldn't need to pass `--auto-commit` on every invocation. The `.yoyo.toml` config file is the right place for per-project preferences.

## Implementation

### src/config.rs

1. Add `"auto_commit"` to `SETTABLE_KEYS`:
   ```
   ("auto_commit", "auto-commit file changes after each agent turn (true/false)"),
   ```

2. Add `parse_auto_commit_from_config()` function, following the exact pattern of `parse_auto_watch_from_config()`:
   ```rust
   pub fn parse_auto_commit_from_config(config: &std::collections::HashMap<String, String>) -> bool {
       match config.get("auto_commit").map(|v| v.as_str()) {
           Some("true") => true,
           Some("false") | None => false,
           Some(other) => {
               eprintln!("  warning: invalid auto_commit value '{other}' — using false");
               false
           }
       }
   }
   ```
   Note: `auto_commit` defaults to `false` (unlike `auto_watch` which defaults to `true`).

3. Add validation in `validate_config_value()` — add an `"auto_commit"` arm that validates `"true"` or `"false"`, following the existing `"auto_watch"` arm pattern.

4. Add tests:
   - `auto_commit_defaults_to_false` — empty config returns false
   - `auto_commit_respects_true` — config with "true" returns true
   - `auto_commit_respects_false` — config with "false" returns false
   - `auto_commit_invalid_value_returns_false` — invalid value defaults to false

### src/cli.rs

5. In `parse_args()`, merge the CLI flag with the config value. Find where `auto_commit` is set (around line 464) and change to:
   ```rust
   let auto_commit = args.iter().any(|a| a == "--auto-commit")
       || crate::config::parse_auto_commit_from_config(&file_config);
   ```
   The CLI flag takes precedence (if either is true, auto_commit is on).

6. Add a test: `test_auto_commit_from_config` — verify that when config has `auto_commit = "true"`, the parsed config has `auto_commit: true` even without the CLI flag.

## Verification

- `cargo build` — no warnings
- `cargo test` — all existing tests pass + new tests pass
- `cargo clippy --all-targets -- -D warnings` — clean
