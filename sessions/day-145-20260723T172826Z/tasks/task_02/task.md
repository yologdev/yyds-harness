Title: Improve bash error recovery hints with bounded retry guidance
Files: src/prompt.rs
Issue: none
Origin: planner

Evidence:
- Trajectory: failed_tool_summary.bash_tool_error=12 — 12 bash tool errors across recent sessions
- Trajectory graph pressure: "Bound failing shell commands before retrying — prefer bounded commands with explicit paths and inspect exit output before retrying"
- The retry loop in `src/prompt.rs` (run_prompt_auto_retry at line 1405, run_prompt_auto_retry_with_content at line 1462) constructs retry prompts when tool calls fail
- Current retry prompts may not include specific guidance about bounding commands (adding timeouts, using explicit paths, checking $? immediately)
- Day 114 learning: "A recovery instruction without timing is a tip, not a safety net" — recovery hints need temporal constraints
- Day 114 also added `set -e` guidance and `./script.sh` path hints to bash recovery — but retry prompt construction may not surface these

Edit Surface:
- src/prompt.rs

Verifier:
- cargo build
- cargo test prompt

Fallback:
- If the retry prompt already includes bounded-command guidance (timeout flags, explicit paths, set -e, check $?), mark this task obsolete.
- If the bash errors are all from harness infrastructure (evolve.sh, which is protected), not from agent tool calls, mark this task obsolete — the agent can't fix harness bash errors.

Objective:
Add a concise bash-error-recovery block to the retry prompt in `src/prompt.rs` that guides the agent to bound commands with timeouts, use explicit paths, and check exit codes immediately — converting recovery tips into actionable constraints.

Why this matters:
The trajectory reports 12 bash tool errors as the #3 graph-derived pressure. Each failed bash command wastes a turn and risks cascading failures. The retry loop already detects tool failures and constructs a retry prompt — adding bounded-command guidance there converts generic "try again" into specific "try again with these constraints" that prevent the same failure mode from repeating.

Success Criteria:
- Retry prompts after bash tool failures include at least one bounded-command hint (timeout, explicit path, or set -e)
- The hint is specific to bash failures, not a generic error message
- `cargo build && cargo test prompt` passes
- The retry prompt construction function has a test that verifies the bash hint is present when the error is a bash tool failure

Verification:
- cargo build
- cargo test prompt

Expected Evidence:
- Future sessions with bash tool errors show the retry prompt including bounded-command guidance
- `failed_tool_summary.bash_tool_error` trend declines in subsequent trajectory snapshots
- No regression in retry behavior for non-bash tool failures

Implementation Notes:
- The retry prompt is constructed in `run_prompt_auto_retry` (line 1405) and `run_prompt_auto_retry_with_content` (line 1462) in `src/prompt.rs`
- Add a helper function that detects whether the failed tool was a bash command and returns a bash-specific recovery hint string
- The hint should be concise (~2-3 lines) and include: (1) add -- to separate flags from positional args, (2) use explicit paths like ./script.sh not script.sh, (3) check $? immediately after the failing command
- Integrate the hint into the existing retry prompt text — do not replace the existing error context, just append the bash-specific guidance
- If the existing retry prompt already has a `recovery_hint` or similar field, use that; otherwise add the hint inline
- Keep the change under 30 lines in src/prompt.rs
