Title: Add auto_continue config option and plan-aware continuation
Files: src/config.rs, src/repl.rs, src/commands_plan.rs
Issue: #389

## Context

Issue #389 suggests configurable auto-continuation. After Task 1 raises MAX_AUTO_CONTINUES to 5 and expands the heuristic, this task adds user control via `.yoyo.toml` and increases the limit during `/plan apply` execution (where the user has already reviewed the plan).

## What to do

### 1. Add config parsing in `config.rs`

Add `auto_continue` and `max_auto_continues` to the SETTABLE_KEYS list and parsing logic:

```toml
# .yoyo.toml
auto_continue = true          # enable/disable auto-continuation (default: true)
max_auto_continues = 5        # max follow-ups per user turn (default: 5, range: 0-20)
```

- Add `"auto_continue"` and `"max_auto_continues"` to `SETTABLE_KEYS`
- Add validation in `validate_config_value` — `auto_continue` accepts "true"/"false", `max_auto_continues` accepts integers 0-20
- Add parsing in `parse_config_file` or `load_config_file` to extract these values

### 2. Wire config into `repl.rs`

- Read the config values early in `run_repl` (or where config is loaded)
- Replace the hardcoded `MAX_AUTO_CONTINUES` with a runtime value that checks:
  1. If `auto_continue = false` in config → skip auto-continue entirely (set limit to 0)
  2. If `max_auto_continues = N` in config → use N instead of the default 5
  3. If in plan-apply mode → use `max(config_limit, 10)` as the ceiling
- Add a function `get_max_auto_continues(config: &HashMap<String, String>, in_plan_apply: bool) -> u32` (or similar) that encapsulates this logic. Make it `pub(crate)` for testing.

### 3. Plan-aware continuation in `commands_plan.rs`

- Add a `pub fn is_plan_apply_active() -> bool` function (using an `AtomicBool` like the existing `is_plan_mode()`) that tracks whether the current prompt is a `/plan apply` execution.
- Set this flag to `true` when `/plan apply` starts and back to `false` when it finishes.
- The repl auto-continue loop checks this flag to use the higher limit.

### 4. Tests

- Test `validate_config_value` for the new keys
- Test `get_max_auto_continues` with various config combinations
- Test that plan-apply flag works correctly

## Verification

- `cargo build && cargo test`
- `cargo clippy --all-targets -- -D warnings` clean
- Config parsing handles edge cases (missing keys use defaults)
