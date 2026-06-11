Title: Add crash diagnostics to MCP connection and agent build failures
Files: src/agent_builder.rs
Issue: none
Origin: planner

Objective:
Wire `stash_diagnostic_error` into MCP server connection failures and agent building
failure paths in `src/agent_builder.rs`. These are startup/pre-init failures that
currently produce exit code 1 or 2 crashes with `no` diagnostic key.

Why this matters:
State evidence: `yyds state crashes` shows 10 crashes, all with exit code 1 or 2 and
`no` diagnostic key. The assessment identifies these as "startup/pre-init failures that
exit before any tool fires." The most likely source is MCP server connection failures
and agent initialization errors in `agent_builder.rs`.

Currently, MCP connection failures (line 156–161) print an error to stderr but don't
stash it as a diagnostic. When the agent exits shortly after, the crash record has
no context about what failed. Adding diagnostic stashing at the failure point makes
future crashes traceable.

Success Criteria:
- `cargo build` passes
- `cargo test` passes
- MCP connection failure paths call `stash_diagnostic_error` with the error message
- OpenAPI spec connection failure paths call `stash_diagnostic_error` similarly

Verification:
- cargo build && cargo test
- grep 'stash_diagnostic_error' src/agent_builder.rs (should find new calls)

Expected Evidence:
- After a session where MCP connection fails, `state crashes` shows a diagnostic key
  like "mcp_connect" instead of "no"
- Crash events gain payload.error with the MCP connection error text

Description:

In `src/agent_builder.rs`, add `crate::state::stash_diagnostic_error` calls in the
MCP server connection failure paths:

1. **MCP stdio connection failure** (around line 156–161):
   After the `Err(e)` arm that prints `"✗ mcp: failed to connect to '{mcp_cmd}': {e}"`,
   add:
   ```rust
   crate::state::stash_diagnostic_error(&format!("mcp_connect: {mcp_cmd}: {e}"));
   ```

2. **MCP structured config connection failure** (find the equivalent Err arm for
   `mcp_server_configs` loop — search for "failed to connect" error pattern in that loop):
   Add a similar `stash_diagnostic_error` call with the server name and error.

3. **OpenAPI spec connection failure** (if there's an equivalent failure path in the
   openapi_specs loop):
   Add a similar `stash_diagnostic_error` call.

4. **Pre-flight tool listing failure** (around line 139–143):
   The pre-flight failure is non-fatal (it proceeds to yoagent connect for diagnostics),
   but stashing a diagnostic here helps trace what went wrong:
   ```rust
   crate::state::stash_diagnostic_error(&format!("mcp_preflight: {command}: {e}"));
   ```

Keep changes minimal. Only insert `stash_diagnostic_error` calls at failure points —
do not change control flow, error handling, or other behavior.
