Title: Add remediation hints to bash command timeout errors
Files: src/tools.rs
Issue: none
Origin: planner

Evidence:
- Trajectory log feedback corrective lesson: "commands timed out during the session -> prefer bounded targeted checks and record timeout-specific remediation"
- Trajectory graph pressure: "Bound failing shell commands before retrying — prefer bounded commands with explicit paths and inspect exit output before retrying"
- The existing bash timeout error in src/tools.rs:662-664 returns `ToolError::Failed(format!("Command timed out after {}s", current_timeout.as_secs()))` — a bare error with no remediation guidance
- The bash command already has a built-in retry mechanism (doubles timeout once, line 645-655), but the final timeout error doesn't tell the agent how to avoid the timeout next time
- The bash tool parameter schema (line 409) already documents an optional `timeout` parameter, but the error doesn't remind the agent to use it
- Day 114 learning: "A recovery instruction without timing is a tip, not a safety net" — timeout errors are the worst case: the agent gets told WHAT happened (timeout) but not HOW to prevent it

Edit Surface:
- src/tools.rs

Verifier:
- cargo build
- cargo test tools
- cargo test -- tools

Fallback:
- If the timeout error message already includes remediation hints (add timeout parameter, use bounded commands, check partial output), mark this task obsolete with a code citation.
- If the bash tool doesn't have a timeout path (only the search tool does), mark this task obsolete.

Objective:
Replace the bare timeout error message "Command timed out after {}s" with a message that includes concrete remediation hints, so the agent knows how to adapt the command instead of retrying the same unbounded call.

Why this matters:
When a bash command times out, the agent gets "Command timed out after 300s" — a dead-end error. The agent then retries the same or similar command, wasting another turn. By including remediation hints in the error message itself, the agent gets actionable guidance immediately: add a timeout parameter, break the command into smaller steps, or check partial output for clues.

Success Criteria:
- The timeout error at src/tools.rs:662-664 includes guidance to: (a) add an explicit `timeout` parameter to the bash call, (b) break the command into smaller bounded steps, (c) check partial output for clues before retrying
- The timeout error is still concise — no more than 2-3 lines
- `cargo build && cargo test` passes
- A test verifies the timeout error string contains remediation tokens like "timeout parameter" or "smaller steps"

Verification:
- cargo build
- cargo test tools
- cargo test -- tools

Expected Evidence:
- Future sessions with bash timeouts show the error message including remediation hints
- Agent retries after timeout errors use timeout parameters or smaller commands more frequently
- `failed_tool_summary.bash_tool_error` trend declines

Implementation Notes:
- The change is at src/tools.rs:662-664 in the `Err(_elapsed)` branch
- Replace:
  ```rust
  Err(ToolError::Failed(format!(
      "Command timed out after {}s",
      current_timeout.as_secs()
  )))
  ```
  with something like:
  ```rust
  Err(ToolError::Failed(format!(
      "Command timed out after {}s. Add an explicit timeout parameter (e.g. timeout: 600) for long-running commands, break into smaller bounded steps, or check partial output for clues before retrying the same command.",
      current_timeout.as_secs()
  )))
  ```
- Also update the diagnostic error stash at line 657-661 to include similar guidance
- Add a test in the existing `#[cfg(test)] mod tests` block that constructs a StreamingBashTool with a very short timeout, runs a `sleep` command that exceeds it, and asserts the error message contains remediation tokens
- Keep the change under 20 lines
