Title: Bundle run_repl's 8 positional arguments into a ReplConfig struct
Files: src/repl.rs, src/main.rs
Issue: none

`run_repl` currently takes 8 positional arguments:

```rust
pub async fn run_repl(
    agent_config: &mut AgentConfig,
    agent: &mut yoagent::agent::Agent,
    mcp_count: u32,
    openapi_count: u32,
    continue_session: bool,
    update_available: Option<String>,
    mcp_cli_servers: Vec<String>,
    mcp_server_configs: Vec<crate::cli::McpServerConfig>,
)
```

This follows the same pattern that `dispatch_command` had before Day 58's `DispatchContext` extraction. The wide signature makes every call site fragile and makes it hard to add new REPL-level configuration.

**What to do:**

1. In `src/repl.rs`, define a `ReplConfig` struct:
   ```rust
   pub struct ReplConfig {
       pub mcp_count: u32,
       pub openapi_count: u32,
       pub continue_session: bool,
       pub update_available: Option<String>,
       pub mcp_cli_servers: Vec<String>,
       pub mcp_server_configs: Vec<crate::cli::McpServerConfig>,
   }
   ```
   
   Note: `agent_config` and `agent` stay as separate parameters because they're mutable borrows with different lifetimes — they don't belong in a config struct.

2. Change `run_repl` signature to:
   ```rust
   pub async fn run_repl(
       agent_config: &mut AgentConfig,
       agent: &mut yoagent::agent::Agent,
       config: ReplConfig,
   )
   ```

3. Update all internal uses in `repl.rs` that reference the old parameter names (e.g., `mcp_count` → `config.mcp_count`).

4. Update the call site in `src/main.rs` to construct a `ReplConfig` and pass it.

5. Run `cargo build && cargo test` to verify zero behavior change.

**Sizing:** 2 files, mechanical refactor. The struct definition + signature change + call site update should take ~10 minutes.
