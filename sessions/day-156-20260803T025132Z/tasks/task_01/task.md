Title: Improve bash recovery hints for common DeepSeek session failure patterns
Files: src/tool_wrappers.rs
Issue: none
Origin: planner (refined from harness-seed)

Evidence:
- Trajectory log feedback: bash_tool_error=5 across recent sessions with corrective lesson "prefer bounded commands with explicit paths and inspect exit output before retrying broader checks"
- The RecoveryHintTool in src/tool_wrappers.rs provides post-failure guidance but current hints are generic ("check the exit code") rather than pattern-matched to the specific stderr output
- Five bash failures in recent sessions that could have self-corrected with better recovery hints represent wasted implementation-agent turns
- This is a Rust change verifiable by cargo test, avoiding the evaluator-timeout revert risk that hits Python-script-only tasks

Edit Surface:
- src/tool_wrappers.rs

Verifier:
- cargo test tool_wrappers

Fallback:
- If the RecoveryHintTool or ToolFailureTracker has been significantly restructured since this plan was written, narrow to adding a single well-tested recovery-hint helper rather than refactoring the existing hint system. If bash recovery hints already cover all common patterns, add a test proving coverage and mark complete.

Objective:
Add pattern-matched recovery hints to the bash tool's failure path so that when a bash command fails with a recognizable error pattern (command not found, path not found, permission denied, timeout, git merge conflict), the recovery hint names the specific fix rather than giving generic advice.

Why this matters:
Implementation agents burn turns retrying bash commands that fail for predictable reasons — missing `./` prefix on scripts, stale file paths, timeout without incrementing the timeout parameter. Each retry costs a full agent turn (~30-60s). Five such failures per session add 2-5 minutes of wasted work. Pattern-matched hints convert those retries into self-corrections on the very next turn.

Success Criteria:
- At least 3 new pattern-matched recovery hints are added (e.g., "command not found" → "use ./script.sh for local scripts", "No such file or directory" → "the file path may have changed; use ls to verify", "timed out" → "add --timeout N to increase the limit")
- Existing recovery hints continue to work
- All tests in src/tool_wrappers.rs pass (or new tests are added for the new hints)

Verification:
- cargo test tool_wrappers
- cargo build

Expected Evidence:
- Future sessions show fewer bash_tool_error counts in log feedback (target: ≤2 instead of 5)
- Implementation transcripts show recovery hints being surfaced for pattern-matched failures
- No regression in existing tool wrapper tests

Implementation Notes:
- RecoveryHintTool is defined in src/tool_wrappers.rs. Search for "RecoveryHint" to find the hint-generation logic.
- The hints are triggered after a tool failure. The failure's stderr/stdout is available for pattern matching.
- Keep hints terse (one line). The goal is actionable, not comprehensive.
- Add at minimum these patterns:
  1. "command not found" / "not found" in stderr → check PATH, use ./ for local scripts
  2. "No such file or directory" → the target path doesn't exist; verify with ls
  3. Timed out / "timed out" → increase the timeout parameter or simplify the command
- Do not modify the tool trait signature — add hint logic within the existing wrapper.
