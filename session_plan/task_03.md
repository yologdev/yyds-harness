Title: Extract config-file parsing functions from cli.rs into config.rs
Files: src/cli.rs, src/config.rs
Issue: none

`cli.rs` is the largest source file at 3,008 lines. It mixes argument parsing (its core job) with config-file loading, path resolution, and TOML parsing. `config.rs` already exists (967 lines) and handles permissions, directories, and MCP server config. This task moves the config-file-related functions from `cli.rs` into `config.rs` where they naturally belong.

**What to move:**

Functions to extract from `cli.rs` → `config.rs`:
- `user_config_path()` — resolves path to `.yoyo.toml`
- `home_config_path()` — resolves path to `~/.config/yoyo/config.toml`
- `history_file_path()` — resolves path to history file
- `parse_config_file()` — reads and parses a TOML config file
- `load_config_file()` — loads config from default locations with fallback

**How to do it:**

1. Move the listed functions from `cli.rs` to `config.rs`, making them `pub` if they aren't already.

2. Update `cli.rs` to import and call the moved functions from `config.rs`:
   - `use crate::config::{user_config_path, home_config_path, history_file_path, parse_config_file, load_config_file};`
   - Or use `config::` prefix at call sites.

3. Update any other files that call these functions to use the new path (check `src/repl.rs`, `src/main.rs`, `src/setup.rs` — they may reference `cli::user_config_path` etc.).

4. Run `cargo build && cargo test` — all existing tests must pass unchanged. No behavioral changes.

**What NOT to move:**
- `parse_args()` and argument-related functions stay in `cli.rs` (that's its core job)
- `Config` struct stays in `cli.rs` (it's the parsed CLI config)
- Help text functions stay in `help.rs`
- `resolve_system_prompt()` stays in `cli.rs` (it's part of arg processing)

**Size estimate:** ~100-150 lines moved. `cli.rs` drops to ~2,850-2,900 lines. `config.rs` grows to ~1,100 lines. Both files stay well under the "too big" threshold.

**Verification:** `cargo build && cargo test` must pass. `grep -rn "cli::user_config_path\|cli::home_config_path\|cli::history_file_path\|cli::parse_config_file\|cli::load_config_file" src/` should return zero hits after migration.
