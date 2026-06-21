Title: Add recovery hints for file-not-found, permission-denied, and spawn-failure tool errors
Files: src/tool_wrappers.rs
Issue: none
Origin: planner

Evidence:
- Trajectory: 8/29 tool failures unrecovered. The Day 112 targeted recovery hints (bash exit-code, search regex) cover only two failure classes.
- Assessment bug #2 (LOW): "8/29 tool failures unrecovered. The recovery hints are scoped to bash exit-code and search regex patterns — failures outside those categories (e.g., file-not-found, permission denied, spawn failures) may lack surface-specific hints."
- Assessment bug #3 (LOW): "Bash tool lacks bounded-command enforcement — AutoCheckTool retries on failure but retries the same unbounded command."
- Graph-derived next-task pressure #1: "Bound failing shell commands before retrying (bash_tool_error=4)."
- The targeted_recovery_hint function in src/tool_wrappers.rs (line 994) currently handles: bash exit codes (non-zero exit with command output), search regex errors (invalid regex patterns), and a generic fallback. Missing patterns: file-not-found (ENOENT, "No such file"), permission-denied (EACCES, "Permission denied"), and spawn-failure (command not found, "No such file or directory" for executables).

Edit Surface:
- src/tool_wrappers.rs — the targeted_recovery_hint function (~line 994-1044) and any related helper

Verifier:
- cargo build && cargo test -- --test-threads=1

Fallback:
- If the implementation agent finds that these error patterns are already handled by targeted_recovery_hint (the assessment may be stale), add a test confirming the coverage and stop. Do not add redundant hint branches.

Objective:
Expand the targeted_recovery_hint function to recognize and provide actionable recovery hints for file-not-found (ENOENT), permission-denied (EACCES), and spawn-failure (command-not-found) tool errors, reducing the 8/29 unrecovered failure rate.

Why this matters:
When the AutoCheckTool retries a failed command, it re-runs the same command. If the failure is a missing file, bad permissions, or a missing executable, the retry will fail identically. A recovery hint that tells the agent "check the file path" or "use ls to verify the file exists" converts a blind retry into an informed correction. Day 112's pipefail and search hints were the first wave; this extends coverage to the next most common failure classes.

Success Criteria:
- targeted_recovery_hint returns a specific, actionable hint for:
  - "No such file or directory" errors (suggest checking path, using ls, or verifying the file exists)
  - "Permission denied" errors (suggest chmod, checking file ownership, or using a different path)
  - "command not found" / spawn-failure errors (suggest which <cmd>, checking PATH, or installing the tool)
- Each hint is ≤200 chars to fit in tool output without overwhelming the agent.
- Existing hints for bash exit-code and search regex continue to work.
- Unit tests verify each new hint pattern against representative error strings.

Verification:
- cargo build && cargo test -- --test-threads=1
- The implementation agent should add focused unit tests in src/tool_wrappers.rs for each new hint pattern, similar to existing tests around line ~2085+.

Expected Evidence:
- Future trajectory shows reduced unrecovered tool failure rate (below 8/29).
- Future sessions show the recovery hint text appearing in tool error output for file-not-found, permission-denied, and spawn-failure errors.
- Dashboard tool-failure recovery metrics improve.

Implementation Notes:
- The targeted_recovery_hint function signature is: fn targeted_recovery_hint(tool_name: &str, error_msg: &str) -> Option<String>
- Match on error_msg substrings (case-insensitive where appropriate):
  - "No such file or directory" → hint about checking the path
  - "Permission denied" → hint about permissions
  - "command not found" or "No such file or directory" (for executables) → hint about PATH/installation
- The RecoveryHintTool already enriches error messages with these hints in its call method (~line 1089-1102).
- Do not modify the AutoCheckTool retry behavior — only add hints. The bounded-command enforcement (graph pressure #1) is a separate concern for a future task.
- The file-not-found pattern should distinguish between read_file failures (suggest list_files to discover the correct path) and bash command failures (suggest ls or find to verify the file).
