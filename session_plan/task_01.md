Title: /config set — persistent runtime configuration
Files: src/commands_config.rs, src/config.rs, src/dispatch.rs
Issue: none

## What

Add `/config set <key> <value>` that persists configuration changes to `.yoyo.toml` (project-local) or `~/.yoyo.toml` (user-level). This closes a direct gap with Claude Code which has `/config` that persists to `settings.json` with project/local/policy layers.

## Why

Currently `/model`, `/provider`, `/think` all work within a session but are lost on restart. Users who prefer a non-default model must pass `--model` every time or manually edit `.yoyo.toml`. Claude Code's `/config` command persists these at runtime.

## Implementation

### 1. Add `write_config_value` to `src/config.rs`
Add a function that reads the existing `.yoyo.toml` (or creates it), updates a single key, and writes it back. Use `toml_edit` crate if available, otherwise do simple string manipulation since our config is flat TOML (no nested tables for the keys we care about).

Supported keys to start:
- `model` — string, the model name
- `provider` — string, the provider name  
- `thinking` — string, thinking level (none/low/medium/high)
- `temperature` — float
- `max_tokens` — integer
- `max_turns` — integer

Function signature:
```rust
pub fn write_config_value(key: &str, value: &str, project_local: bool) -> Result<(), String>
```

If `project_local` is true, write to `.yoyo.toml` in current dir. Otherwise write to `~/.yoyo.toml`.

Simple approach: read file, check if key exists (regex `^key\s*=`), replace line or append. No need for a full TOML parser — our config files are simple.

### 2. Add `handle_config_set` to `src/commands_config.rs`
Parse input after `/config set`. Format: `/config set <key> <value> [--global]`.
- Without `--global`: writes to `.yoyo.toml` (project-local)
- With `--global`: writes to `~/.yoyo.toml`

After writing, also update the live runtime config (AgentConfig) so the change takes effect immediately.

Print confirmation: `  ✓ Set model = claude-sonnet-4-20250514 in .yoyo.toml`

Also add `/config get <key>` to show a single value.

### 3. Route in `src/dispatch.rs`
The `/config` command already routes to `handle_config`. Add subcommand detection:
- `/config set <key> <value>` → `handle_config_set`
- `/config get <key>` → `handle_config_get`
- `/config` (no args) → existing `handle_config` (show all)

### Tests
- Test `write_config_value` creates new file with key
- Test `write_config_value` updates existing key
- Test `write_config_value` preserves other keys
- Test `handle_config_set` with valid and invalid keys
- Test parsing of `/config set` arguments

### Docs
- Update `docs/src/configuration/` if it exists with info about `/config set`
- Add `/config set` and `/config get` to help text
