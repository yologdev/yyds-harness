Title: Extract agent builder module from main.rs
Files: src/main.rs, src/agent_builder.rs
Issue: none

## What to do

Extract the agent-building logic from `main.rs` (2,484 lines) into a new `src/agent_builder.rs` module.
`main.rs` currently does multiple jobs: module declarations, entry point (`main`), agent configuration
(`AgentConfig`, `build_agent`, `build_side_agent`), model config creation (`create_model_config`),
MCP collision detection, fallback retry logic, and various helper functions.

### What to extract

Move these items into `src/agent_builder.rs`:
- `struct AgentConfig` and its `impl` block (including `build_agent`, `build_side_agent`)
- `create_model_config` function
- `insert_client_headers` helper
- `yoyo_user_agent` helper
- `FallbackRetry` enum and `try_fallback_prompt` function
- `BUILTIN_TOOL_NAMES` constant and `detect_mcp_collisions` function
- `fetch_mcp_tool_names` and `connect_external_servers` async functions

### What stays in main.rs

- `mod` declarations
- `main()` function
- `run_single_prompt`, `run_piped_mode`, `looks_like_slash_command`
- CLI flag application helpers (`apply_cli_flags`, `apply_config_flags`)
- Setup/restore helpers (`run_setup_wizard_if_needed`, `apply_bedrock_credentials`, `restore_session`)
- `build_json_output`

### Implementation notes

1. Create `src/agent_builder.rs` with the extracted items.
2. Add `mod agent_builder;` to `main.rs`.
3. Make extracted items `pub(crate)` so they're accessible from `main.rs` and other modules.
4. Update `use` statements in `main.rs` to import from `agent_builder`.
5. Check all other files that reference `BUILTIN_TOOL_NAMES`, `detect_mcp_collisions`, `create_model_config`,
   or `AgentConfig` — update their imports. Search with `grep -rn "BUILTIN_TOOL_NAMES\|detect_mcp_collisions\|create_model_config\|AgentConfig" src/`.
6. Keep the `#[cfg(test)] mod tests` for `detect_mcp_collisions` — move them into `agent_builder.rs` too.

### Acceptance criteria
- `main.rs` drops by ~800-1000 lines
- `cargo build && cargo test && cargo clippy --all-targets -- -D warnings` all green
- No behavioral change — pure structural extraction
- Update CLAUDE.md's "Architecture" section to mention the new `agent_builder.rs` module
