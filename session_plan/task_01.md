Title: Fix silent .ok() data loss in save_messages across provider/model/thinking switches
Files: src/commands.rs, src/commands_config.rs, src/dispatch.rs
Issue: none

## Problem

When users switch models (`/model`), providers (`/provider`), or thinking levels (`/think`), the agent rebuilds itself and attempts to preserve the conversation via `agent.save_messages().ok()`. The `.ok()` silently discards any error, meaning if serialization fails, the user loses their entire conversation history with zero warning.

This is the same pattern fixed in Day 68 for `main.rs`, `prompt.rs`, and `commands_retry.rs`. These 5 remaining call sites were identified but not yet fixed.

## Locations

1. `src/commands.rs:430` — `handle_provider_switch`
2. `src/commands_config.rs:634` — `handle_config_set` for "model" key
3. `src/commands_config.rs:646` — `handle_config_set` for "thinking" key
4. `src/dispatch.rs:195` — `/model` command handler
5. `src/dispatch.rs:242` — `/think` command handler

## Fix

Replace each `let saved = agent.save_messages().ok();` with:

```rust
let saved = match agent.save_messages() {
    Ok(json) => Some(json),
    Err(e) => {
        eprintln!("{DIM}  ⚠ could not preserve conversation: {e}{RESET}");
        None
    }
};
```

This matches the pattern established in Day 68. The user sees a warning but the operation (model/provider switch) still proceeds — it just starts with a fresh conversation.

## Verification

- `cargo build && cargo test` must pass
- Grep for remaining `save_messages().ok()` to confirm none are left (zero remaining after this fix)
