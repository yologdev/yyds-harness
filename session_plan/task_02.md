Title: Auto-lite detection and .yoyo.toml config support
Files: src/cli.rs, src/cli_config.rs, src/config.rs
Issue: #415

## Description

When a user sets a small context window (≤16K tokens) via `--context-window`, automatically enable lite-mode behaviors without requiring `--lite`. Also add `lite = true` support in `.yoyo.toml` config file so users with local models don't have to pass the flag every time.

### What to implement:

1. **In `src/cli.rs` (parse_args):**
   - After all flags are parsed, add auto-lite detection logic:
     ```rust
     // Auto-enable lite mode when context window is very small
     if !config.lite {
         if let Some(cw) = config.context_window {
             if cw <= 16_000 {
                 // Apply lite defaults (same as --lite) but don't override explicit user choices
                 config.lite = true;
                 // Only override system_prompt if user didn't pass --system-prompt
                 if config.system_prompt == SYSTEM_PROMPT {
                     config.system_prompt = LITE_SYSTEM_PROMPT.to_string();
                 }
                 // Only set disallowed_tools if user didn't pass --disallowed-tools
                 if config.disallowed_tools.is_empty() {
                     config.disallowed_tools = compute_lite_disallowed_tools();
                 }
             }
         }
     }
     ```
   - Extract the lite-mode disallowed tools computation into a helper function `compute_lite_disallowed_tools() -> Vec<String>` so both `--lite` and auto-detection share the same logic.

2. **In `src/config.rs`:**
   - In `parse_config_file` (or the appropriate config loading function), parse a `lite = true` key from the `[general]` or top-level section of `.yoyo.toml`
   - Add `"lite"` to `SETTABLE_KEYS` so `/config set lite true` works
   - Add validation in `validate_config_value` for the `lite` key (must be "true" or "false")

3. **In `src/cli.rs`:**
   - When loading config from `.yoyo.toml`, apply the `lite` setting (same precedence as other config: CLI flag overrides file)
   - Print a dim notice when auto-lite activates: `"  🪶 Auto-lite: context window ≤16K, using minimal tool set"`

### Tests to add:
- Test that `--context-window 8000` triggers auto-lite (sets lite=true, disallowed_tools populated)
- Test that `--context-window 32000` does NOT trigger auto-lite
- Test that `--context-window 8000 --disallowed-tools bash` does NOT override the user's explicit disallowed_tools
- Test that `validate_config_value("lite", "true")` passes and `validate_config_value("lite", "banana")` fails
