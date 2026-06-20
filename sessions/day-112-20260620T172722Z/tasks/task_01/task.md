Title: Harden bash tool with pipefail and bounded command prefix
Files: src/tools.rs
Issue: none
Origin: planner

Evidence:
- Graph pressure #1: bash_tool_error=12 — highest-count recurring tool failure category in state evidence.
- `git grep -n pipefail src/tools.rs` returns zero matches. The `StreamingBashTool::execute` method at line 477 runs `bash -c <command>` without `set -o pipefail`, so pipe failures (e.g., `rg ... | head -20` where rg exits non-zero) are silently masked.
- State events show 12 bash tool errors across sessions; many are exit-code failures from piped commands where a non-zero exit from the first command was hidden by the last command's exit code.
- Recent action evidence: no current-session reproduction, but the category is the largest single failure bucket.

Edit Surface:
- src/tools.rs (StreamingBashTool::execute, around line 475-478)

Verifier:
- cargo test --bin yyds -- --test-threads=1
- cargo test --test integration -- --test-threads=1

Fallback:
- If `cargo test` fails and the fix loop exhausts, revert. Do not touch scripts/evolve.sh or other files.

Objective:
Make every bash invocation via the agent's bash tool run with `set -o pipefail` so that non-zero exits in piped commands propagate to the tool result. This prevents the agent from trusting partial output when a pipe member fails silently.

Why this matters:
The bash tool is the agent's primary interaction mechanism. When a pipeline like `rg pattern src/ | head -20` fails because rg exits with code 2 (error), but head exits 0, the agent sees partial/empty output and proceeds under false assumptions. `set -o pipefail` ensures the first non-zero exit in a pipeline becomes the overall exit code, surfacing the real failure to the agent for retry or diagnosis. This is the highest-count recurring tool failure (12 events), and each one wasted evolution turns.

Success Criteria:
- `set -o pipefail;` is prepended to every bash command before execution.
- Existing bash tool tests pass without modification (or minimal adaptation if they test exit-code behavior).
- Piped commands that fail mid-pipeline now report the actual failure exit code to the agent.

Verification:
- `cargo test tools` — StreamingBashTool tests
- `cargo test --bin yyds -- --test-threads=1`
- `cargo test --test integration -- --test-threads=1`

Expected Evidence:
- In future state events, bash_tool_error count decreases.
- Agent retry loops trigger on genuine pipe failures instead of proceeding with partial output.
- Dashboard tool-failure reconciliation shows fewer bash exit-code failures.

## Implementation Notes

In `StreamingBashTool::execute` (around line 475-478), the effective command is run via:
```rust
let mut cmd = tokio::process::Command::new("bash");
cmd.arg("-c").arg(&effective_command);
```

Change this to prepend `set -o pipefail;` to the command:
```rust
let guarded_command = format!("set -o pipefail; {}", effective_command);
let mut cmd = tokio::process::Command::new("bash");
cmd.arg("-c").arg(&guarded_command);
```

Also add `set -e` as a safety companion? No — `set -e` is too aggressive for agent workflows where intermediate failures in multi-command scripts (e.g., `mkdir -p; cd; cargo build`) should not abort the whole script. Use `set -o pipefail` only.

Do NOT modify `maybe_prefix_rtk` or any other function. The pipefail prefix should be applied at the command construction site (line 475-478), after RTK prefixing but before spawning.

If test failures arise from tests that expect specific exit-code behavior from pipelines, update the test assertion to reflect pipefail semantics (a pipe member failing mid-pipeline should produce a non-zero exit code and potentially a ToolError).
