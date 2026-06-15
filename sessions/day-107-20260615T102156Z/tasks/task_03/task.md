Title: Investigate and fix "Tool grep not found" errors
Files: src/commands_search.rs, src/tools.rs
Issue: none
Origin: planner

Objective:
Diagnose and fix the "Tool grep not found" error that appears in recent state failure records (3 instances in current window). Determine whether this is a tool-routing bug, a CI environment issue, or a tool-name collision, and apply the appropriate fix.

Why this matters:
Assessment evidence: 3 recent instances of "Tool grep not found" errors in the failure log. Recent tool failures from `state failures --recent` show this pattern alongside timeouts and missing-parameter errors.

When a tool the agent tries to use isn't found, agent turns fail and retries consume context budget. If this is a code-level tool routing issue (e.g., the `search` tool aliasing or the `grep` subcommand not being properly registered), it degrades every session. If it's a CI environment issue (grep binary missing in GitHub Actions), it can be fixed with a dependency check. If it's a tool-name collision with an MCP server, the MCP collision guard should have caught it.

The assessment also notes: "grep-not-found — 3 recent. May be a tool routing issue worth investigating."

Success Criteria:
- Root cause identified: tool routing bug, CI environment, or MCP collision
- Fix applied or documented with clear next steps if the fix requires changes outside the 2-file scope
- `state failures --recent` no longer shows "Tool grep not found" after the fix is verified
- `cargo test` passes

Verification:
- cargo build && cargo test
- If fix is in tool registration: verify the grep/search tool works with a test invocation
- If fix is in CI: verify grep is available in the environment
- Check `state failures --recent` after fix to confirm no new instances

Expected Evidence:
- "Tool grep not found" count drops to 0 in subsequent session failure records
- If a tool-registration fix: the affected tool path now resolves correctly
- If a CI fix: grep dependency is checked/ensured before tool invocation

Implementation Notes:
- First, check how the grep tool is invoked. In `commands_search.rs`, look for how `/grep` or the `search` tool calls the system `grep` binary. The "not found" error could mean:
  a. The `grep` binary path is hardcoded and wrong
  b. `grep` is not installed in the CI environment (GitHub Actions ubuntu-latest has grep by default, but custom runners might not)
  c. A tool-name collision with an MCP server is incorrectly routing grep calls
- In `tools.rs`, check the builtin tool list for any tool named "grep" that might shadow the system command.
- The error might also come from the `search` tool (yoagent's built-in search tool) rather than a bash `grep` invocation. Check if the error message format matches yoagent's tool-not-found error vs. a bash "command not found" error.
- If the root cause is environmental (grep missing), add a startup check that warns or installs it.
- Keep investigation focused — if the cause spans more than the 2 listed files, document findings and create a follow-up task.
