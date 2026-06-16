Title: Enforce default timeout on StreamingBashTool to reduce bash_tool_errors
Files: src/tools.rs, src/cli_config.rs
Issue: none
Origin: planner

Objective:
Add a configurable default timeout to `StreamingBashTool` so that bash commands without an explicit `timeout` parameter are bounded. This reduces the 6 `bash_tool_error` events flagged in trajectory graph pressure by preventing commands from hanging indefinitely or exceeding reasonable runtimes.

Why this matters:
The trajectory graph pressure reports `bash_tool_error=6` — bash commands failing in recent sessions. The graph suggestion says "prefer bounded commands with explicit paths and inspect exit output." Currently `StreamingBashTool` has no timeout enforcement: if an agent forgets to pass `timeout`, the command runs unbounded. Adding a default timeout (e.g., 300s) ensures every bash command has a ceiling, making failures faster and more diagnosable. This directly addresses the highest-ranked graph-pressure row that is actionable in source code (CI fingerprint issues are in protected `.github/workflows/`).

The yoagent bash tool already supports a `timeout` parameter — this task adds the harness-level default that kicks in when the agent doesn't specify one.

Success Criteria:
- Bash commands without an explicit `timeout` parameter are capped at a default timeout (300s)
- Commands with an explicit `timeout` parameter respect the caller's value (no override)
- The default timeout value is a constant in `src/cli_config.rs` for discoverability
- Existing tests pass: `cargo test --bin yyds -- --test-threads=1 tools`
- `StreamingBashTool::default()` includes the timeout in its tool description/parameters so the agent sees it

Verification:
- cargo check
- cargo test --bin yyds -- --test-threads=1 tools
- cargo test --bin yyds -- --test-threads=1 cli_config

Expected Evidence:
- Future trajectory reports should show reduced `bash_tool_error` counts
- Tool call state events (`CommandCompleted`) for timed-out commands should show the timeout in their metadata
- No regression in agent behavior — commands that explicitly set timeout continue working

Implementation Notes:
- Add `DEFAULT_BASH_TIMEOUT_SECS: u64 = 300` to `src/cli_config.rs` constants
- In `src/tools.rs`, modify `StreamingBashTool` to store an `Option<u64>` default timeout
- In `StreamingBashTool::call` or the bash execution path, when the agent's `timeout` parameter is absent or zero, apply the default
- The timeout should be applied at the subprocess level (pass `timeout` to the bash invocation or use tokio::time::timeout)
- Do not change the tool description format unnecessarily — just ensure the parameter schema documents the default
- If the yoagent `BashTool` already provides a default-timeout builder method, prefer that over reimplementing the timeout wrapping
