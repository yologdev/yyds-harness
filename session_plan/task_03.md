Title: Extract CLI constants and Config struct from cli.rs into cli_config.rs
Files: src/cli.rs, src/cli_config.rs, src/main.rs
Issue: none

## Context

`cli.rs` is the largest source file at 2,869 lines. It mixes several concerns:
- Constants (VERSION, DEFAULT_CONTEXT_TOKENS, thresholds, SYSTEM_PROMPT, DEFAULT_SESSION_PATH, etc.)
- The `Config` struct and its fields
- The `ContextStrategy` and `OutputFormat` enums
- verbose/quiet state management (enable_verbose, is_verbose)
- Banner/welcome display
- Argument parsing (parse_args and helpers)

The assessment identifies this as a split candidate. Following the grain-of-reorganization pattern (Day 65), this is an expression-level extraction — moving constants and the Config type to improve readability.

## What to do

### 1. Create `src/cli_config.rs`

Move these items from `cli.rs` to `cli_config.rs`:
- `VERSION` const
- `DEFAULT_CONTEXT_TOKENS`, `AUTO_COMPACT_THRESHOLD`, `PROACTIVE_COMPACT_THRESHOLD` consts
- `set_effective_context_tokens()` and `effective_context_tokens()` functions with their static
- `DEFAULT_SESSION_PATH`, `AUTO_SAVE_SESSION_PATH` consts
- `SYSTEM_PROMPT` const
- `ContextStrategy` enum (with derives)
- `OutputFormat` enum (with derives)
- `Config` struct (with all fields)

### 2. Update `cli.rs`

- Add `mod cli_config;` to the appropriate place (or in main.rs)
- Re-export everything from `cli_config` via `pub use cli_config::*;` in `cli.rs` so that downstream consumers don't break
- Remove the moved items from `cli.rs`

### 3. Update `main.rs`

- Add `mod cli_config;` declaration if needed
- Ensure all existing `use crate::cli::*` paths still work (they should via the re-export)

## Goal

Reduce `cli.rs` by ~200-300 lines. The re-export ensures zero downstream breakage — every file that imports from `cli` still works.

## Tests

- All existing tests must pass unchanged
- Add `test_cli_config_constants` in cli_config.rs to verify constants are accessible and have expected values
- Add `test_effective_context_tokens_roundtrip` to verify the set/get pattern works

## What NOT to do

- Don't move parse_args or any parsing logic — that stays in cli.rs
- Don't move banner/welcome functions — they're display logic that belongs with CLI
- Don't change any public API signatures
- Don't rename any constants or types
