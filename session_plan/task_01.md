Title: Extract MCP/OpenAPI connection setup from main() into a helper function
Files: src/main.rs
Issue: none

## What to do

The `main()` function in `main.rs` is 1,479 lines — the largest function in the codebase. This task extracts the MCP and OpenAPI connection setup into a dedicated helper function, removing ~150 lines from `main()`.

### Specifically:

1. Create a new async function `connect_external_servers()` (or similar) that takes:
   - `&AgentConfig` (for rebuilding agent on failure)
   - `Agent` (consumed and returned — the with_mcp_server_stdio pattern)
   - `&[String]` for mcp_servers (the --mcp flag values)
   - `&[McpServerConfig]` for mcp_server_configs (the TOML-configured servers)
   - `&[String]` for openapi_specs
   - Returns `(Agent, u32, u32)` — the updated agent plus mcp_count and openapi_count

2. This function should contain:
   - The `for mcp_cmd in &mcp_servers` loop (lines ~922-970) that does pre-flight collision checking and connects --mcp servers
   - The `for server_cfg in &mcp_server_configs` loop (lines ~975-1035) that connects TOML-configured MCP servers
   - The `for spec_path in &openapi_specs` loop (lines ~1037-1055) that loads OpenAPI specs

3. Replace all three loops in `main()` with a single call to the new function.

4. Update any existing tests if they reference the moved code. The `detect_mcp_collisions` and `fetch_mcp_tool_names` functions stay where they are — only the connection loops move.

### Verification:
- `cargo build` must pass
- `cargo test` must pass (all 85 tests)
- `cargo clippy --all-targets -- -D warnings` must pass

This is the first step in decomposing main() — scoped to one self-contained block (external server connections) that has no interleaving with other logic.
