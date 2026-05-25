Title: Persist thinking/no_bell/quiet/no_color in .yoyo.toml config
Files: src/config.rs, src/cli.rs
Issue: none

## Description

Currently `thinking`, `no_bell`, `quiet`, and `no_color` only work as CLI flags (`--thinking high`, `--no-bell`, `--quiet`, `--no-color`). Users must pass them every time. Make these persistable in `.yoyo.toml` so config-file values are used as defaults, with CLI flags overriding.

### What to implement

**In `config.rs`:**
1. Add `parse_thinking_from_config(config: &HashMap<String, String>) -> Option<String>` — reads `thinking` key, returns the level string if present and valid.
2. Add `parse_no_bell_from_config(config: &HashMap<String, String>) -> bool` — reads `no_bell` key, returns true if `"true"`.
3. Add `parse_quiet_from_config(config: &HashMap<String, String>) -> bool` — reads `quiet` key, returns true if `"true"`.
4. Add `parse_no_color_from_config(config: &HashMap<String, String>) -> bool` — reads `no_color` key, returns true if `"true"`.
5. Add `"no_bell"`, `"quiet"`, and `"no_color"` to `SETTABLE_KEYS` (with descriptions like "suppress terminal bell (true/false)").
6. Add validation for these keys in `validate_config_value` — they should accept true/false.

**In `cli.rs` / `parse_args`:**
1. `thinking` is already read from config (line ~796). No change needed there.
2. For `no_bell`: if CLI flag `--no-bell` is NOT present, check `parse_no_bell_from_config(&file_config)`. If true, call `disable_bell()`.
3. For `quiet`: if CLI flags `--quiet`/`-q` are NOT present, check `parse_quiet_from_config(&file_config)`. If true, call `enable_quiet()`.
4. For `no_color`: if CLI flag `--no-color` is NOT present, check `parse_no_color_from_config(&file_config)`. If true, call `disable_color()`.

The CLI flag should always win (override config). The config file is the default.

### Tests to add

In `config.rs` tests:
- Test `parse_no_bell_from_config` returns true/false correctly.
- Test `parse_quiet_from_config` returns true/false correctly.
- Test `parse_no_color_from_config` returns true/false correctly.
- Test `validate_config_value` accepts true/false for new keys.
- Test that `SETTABLE_KEYS` contains the new entries.

### Example .yoyo.toml after

```toml
provider = "anthropic"
model = "claude-sonnet-4-20250514"
thinking = "high"
no_bell = true
quiet = false
no_color = false
```

### Verification
`cargo build && cargo test && cargo clippy --all-targets -- -D warnings`
