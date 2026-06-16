Title: Improve bash tool error recovery with bounded-command hints
Files: src/tools.rs, src/prompt_retry.rs
Issue: none
Origin: planner

Objective:
Reduce bash tool failure impact by surfacing exit-code and bounded-command advice when a StreamingBashTool command fails.

Why this matters:
Graph-derived next-task pressure row #1: `failed_tool_summary.bash_tool_error=3` — "Bound failing shell commands before retrying: prefer bounded commands with explicit paths and inspect exit output before retrying broader checks." Log feedback corrected lesson: "shell tool commands failed during the session → prefer bounded commands with explicit paths and inspect exit output before retrying broader checks."

Three bash tool errors occurred in recent sessions. When a bash command fails (non-zero exit), the current tool output includes stdout/stderr but doesn't clearly surface:
- The exit code as a structured field
- A hint to retry with a bounded/qualified command (explicit paths, `--` flags, quoted args)
- Whether the failure looks transient (network, timeout) vs permanent (file not found, syntax error)

These hints help the agent recover faster and reduce the retry loop cost.

Success Criteria:
- StreamingBashTool result includes a clear exit-code line when `exit_code != 0` (e.g., "Exit code: 1").
- The retry classification in `prompt_retry.rs` already detects "bash error:" at line 1126 — verify it works with the new format.
- A non-zero exit triggers a bounded-command hint in the tool output (e.g., "Tip: retry with explicit paths and -- to separate flags from args").
- Existing tests pass; add a unit test for the exit-code formatting.

Verification:
- cargo test tools:: -- --test-threads=1
- cargo test prompt_retry:: -- --test-threads=1
- cargo clippy --all-targets -- -D warnings
- cargo check

Expected Evidence:
- Future sessions show fewer bash tool errors in the failed_tool_summary metric.
- When bash commands do fail, the agent's retry output shows it using bounded/qualified commands.
- State events for ToolCallCompleted on bash failures include the exit code in payload.

Implementation Notes:
- The StreamingBashTool's `execute` method (around line 107 in `src/tools.rs`) already captures exit status from the subprocess. Add a post-execution formatting step that appends exit-code info and a bounded-command hint to the output when `exit_code != 0`.
- The hint should be short (1-2 lines) and actionable, not a wall of text. Example: "Exit code: 2. Tip: use explicit paths (./script.sh, not script.sh) and -- to separate flags from positional args."
- In `src/prompt_retry.rs`, around line 1126, the retry classification checks for "bash error:" — verify the new output format still matches this pattern, or update the pattern.
- Do NOT change the confirm/safety gate behavior — those are separate concerns.
