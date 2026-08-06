Title: Add recovery hints for common bash failure patterns beyond signal-kill
Files: src/tool_wrappers.rs
Issue: none
Origin: planner

Evidence:
- Trajectory graph pressure: "Bound failing shell commands before retrying
  (failed_tool_summary.bash_tool_error=4): prefer bounded commands with
  explicit paths and inspect exit output before retrying"
- Day 158 Task 2 (commit 46d3f8fe) added signal-kill exit code hints
  (130=SIGINT, 143=SIGTERM, 137=SIGKILL) to `targeted_recovery_hint()` in
  `src/tool_wrappers.rs`. The trajectory still shows `bash_tool_error=4`,
  indicating non-signal bash failures that lack targeted recovery hints.
- The assessment notes these 4 failures are "likely from cancelled-run noise"
  but the graph pressure explicitly requests better recovery hints, which
  benefit interactive use regardless of the failure source.
- The RecoveryHintTool wrapper (`src/tool_wrappers.rs`) already has a
  `targeted_recovery_hint` function that matches on exit codes and stderr
  patterns. Day 158 Task 2 extended it for signal kills. The same function
  can be extended for other common failure classes.

Edit Surface:
- src/tool_wrappers.rs

Verifier:
- cargo test tool_wrappers -- --test-threads=1

Fallback:
- If `targeted_recovery_hint` already covers all the failure patterns listed
  below, write task_02_obsolete.md with the evidence. If the bash_tool_error=4
  in the trajectory is from cancelled-run sessions where commands were killed
  mid-execution (not recoverable failures), note that the hints won't help
  those cases but are still valuable for interactive use.

Objective:
Extend the `targeted_recovery_hint` function to produce helpful recovery
suggestions for common non-signal bash failure patterns, reducing the
bash_tool_error count and improving agent recovery from failed commands.

Why this matters:
Bash is the most-used tool (4,148 edges in the state graph). When a bash
command fails without a targeted hint, the agent retries blindly or abandons
the task. Targeted hints convert generic failures into actionable next steps:
"command not found" → install it, "permission denied" → chmod +x, empty
output → check the path. This directly reduces retry waste and improves
task completion rates.

Success Criteria:
- `targeted_recovery_hint` produces specific recovery suggestions for at least
  3 new failure patterns beyond signal-kill.
- Each new hint is backed by a unit test.
- Existing tests still pass.

Verification:
- cargo test tool_wrappers -- --test-threads=1
- cargo check

Expected Evidence:
- New tests in the tool_wrappers test module verifying recovery hints for:
  exit code 127 (command not found), exit code 126 (permission denied),
  stderr "No such file or directory" (path errors).
- The bash_tool_error count in future trajectory runs may decrease if the
  failures are from recoverable patterns rather than cancelled-run noise.

Implementation Notes:
- The `targeted_recovery_hint` function (or equivalent) in
  `src/tool_wrappers.rs` already handles exit code matching and stderr pattern
  matching for signal-kill codes. Extend it with additional match arms.

Target failure patterns to add (in priority order):

1. **Exit code 127 — "command not found"**: The binary isn't installed or not
   in PATH. Hint: "Command not found. Check with `which <cmd>` or
   `command -v <cmd>`. If missing, install it or use the full path."

2. **Exit code 126 — "Permission denied" / "not executable"**: A script or
   binary exists but isn't executable. Hint: "Permission denied. If this is a
   script, run `chmod +x <path>`. If a binary, check file permissions."

3. **stderr contains "No such file or directory"**: A path in the command
   doesn't exist. Hint: "File or directory not found. Check the path with
   `ls <path>` or `file <path>`. Use absolute paths when possible."

4. **Exit code 124 — timeout**: The command was killed by `timeout`.
   Hint: "Command timed out. Try reducing the scope (limit file reads,
   narrow search paths) or splitting into smaller commands."

5. **stderr contains "Permission denied" (without exit 126)**: Access denied
   to a file or directory. Hint: "Permission denied. Check file ownership
   with `ls -la <path>` and permissions. Avoid paths requiring sudo."

- Each hint should be 1-2 sentences, actionable, and reference concrete
  commands the agent can run to diagnose or fix the problem.
- Follow the existing pattern in the codebase — match on exit code first,
  then stderr patterns as fallback.
- Do NOT modify the bash tool itself (tools.rs) — only the recovery hint
  wrapper in tool_wrappers.rs.
