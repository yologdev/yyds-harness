Title: Wire crash reporter into StreamingBashTool execution failures
Files: src/tools.rs
Issue: none
Origin: planner

Objective:
Wire `state::stash_diagnostic_error()` into the StreamingBashTool's error return paths so that bash command execution failures (spawn errors, timeouts, wait failures) leave diagnostic traces visible via `/state crashes`.

Why this matters:
The crash reporter was wired into 2 doors last session (state init in lib.rs, transport failures in deepseek.rs). The assessment identifies at least 5 more failure sites that vanish without trace. The StreamingBashTool is the most heavily used tool (338 invocations in current state) and its failures (timeout, spawn, wait) indicate genuine operational problems. Wiring it costs ~10-15 lines and follows the proven pattern from Day 103's 9-line transport failure wiring.

Success Criteria:
- `cargo build` and `cargo test` pass
- When a bash command times out, the error detail is stashed (observable via `stash_diagnostic_error` being called before the Err return)
- When a bash command fails to spawn, the error detail is stashed
- When a bash command wait fails, the error detail is stashed
- Cancellation errors are NOT stashed (cancellation is normal operation, not a crash)

Verification:
- `cargo build`
- `cargo test --lib -- tools`
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings 2>&1 | head -20` (sanity check)

Expected Evidence:
- After the change, any StreamingBashTool timeout/spawn/wait errors in future sessions will appear in `/state crashes` output
- The crash-reporter door count increases from 2 to 3 (transport, state-init, tool-execution)
- State events show `stash_diagnostic_error` being called from `tools.rs` path

Description:

In `src/tools.rs`, the `StreamingBashTool::execute()` method (around line 412) has three error-return paths that indicate genuine failures:

1. **Spawn failure** (line ~486): `cmd.spawn().map_err(|e| ToolError::Failed(format!("Failed to spawn: {e}")))?` — This is a genuine infrastructure failure. Before the `?` propagates the error, call `crate::state::stash_diagnostic_error(&format!("bash spawn failed: {e}"))`.

2. **Timeout** (line ~600-606): `return Err(ToolError::Failed(format!("Command timed out after {}s", timeout.as_secs())))` — This is a resource exhaustion failure. Before the return, call `crate::state::stash_diagnostic_error(&format!("bash timeout: {} after {}s", command, timeout.as_secs()))`.

3. **Wait failure** (line ~609): `status.map_err(|e| ToolError::Failed(format!("Failed to wait: {e}")))?` — This is a process-management failure. Before the `?`, call `crate::state::stash_diagnostic_error(&format!("bash wait failed: {e}"))`.

Do NOT wire cancellation paths (line ~598: `return Err(yoagent::types::ToolError::Cancelled)`). Cancellation is normal operation.

Implementation pattern (copy the style from `src/deepseek.rs:1022`):
```rust
crate::state::stash_diagnostic_error(&format!("bash spawn failed: {e}"));
```

The function already imports `use yoagent::types::{Content, ToolError, ToolResult as TR}`. No new imports needed since `crate::state::stash_diagnostic_error` is accessed via full path.
