Title: Add 2-3 recovery hints for uncovered bash error patterns in src/tool_wrappers.rs
Files: src/tool_wrappers.rs
Issue: none
Origin: planner

Evidence:
- Trajectory graph pressure #4: "Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=12): prefer bounded commands with explicit paths and inspect exit output before retrying broader checks."
- Recent shipped recovery hints (Day 164): "Is a directory" → suggest `ls`, "No space left on device" → suggest `df`/`du`/`cargo clean`. Day 160-164 arc added 6+ recovery hints across signal kills, exit code 42, network errors, git errors, directory errors, and disk errors. All shipped cleanly.
- The assessment confirms: "The recovery-hint table has been growing row by row and is becoming a genuine diagnostic assistant." But bash_tool_error=12 across recent sessions suggests patterns remain uncovered.
- The recovery-hint apparatus is a simple match statement in `src/tool_wrappers.rs` — each hint is 3-5 lines of Rust. These are the most reliably shippable tasks in the codebase.

Edit Surface:
- src/tool_wrappers.rs

Verifier:
- cargo test tool_wrappers -- --test-threads=1
- cargo build

Fallback:
- If a review of the existing recovery-hint table shows all common bash error patterns are already covered (the remaining 12 failures are one-offs or harness-shell failures), write an obsolete note and mark the task complete with zero changes.
- If the recovery-hint code has been refactored or moved since the assessment was written, verify the new location before editing.

Objective:
Add 2-3 new recovery hints for bash error patterns that appear in recent session transcripts but aren't yet covered by the existing recovery-hint table. Each hint maps a stderr substring to a concrete diagnostic suggestion.

Why this matters:
Recovery hints are the fastest path from "something broke" to "I know what to check." Each hint saves the agent 2-3 turns of diagnostic flailing. The bash_tool_error=12 signal shows there's still friction. Continuing this arc converts raw failure counts into guided recovery, which directly improves the agent's self-correction throughput. This is the most proven-productive task class in recent sessions — every recovery-hint task shipped on first attempt.

Success Criteria:
- 2-3 new match arms added to the recovery-hint match statement in `src/tool_wrappers.rs`.
- Each hint maps a specific stderr pattern to a concrete suggestion (not generic advice).
- `cargo build && cargo test` pass with no regressions.
- No existing hints are modified or removed.

Verification:
- cargo build
- cargo test -- --test-threads=1
- grep for the new error substrings in the recovery-hint function to confirm they're present.

Expected Evidence:
- Task lineage shows tool_wrappers.rs change that passed strict verification.
- Next trajectory shows reduced bash_tool_error count or more specific failure classification.
- The recovery-hint table grows by 2-3 rows.

Implementation Notes:
- Find the recovery-hint function in `src/tool_wrappers.rs`. Search for existing patterns like "Is a directory" or "No space left on device" to locate the match statement.
- Candidates for new hints (pick 2-3 that aren't already covered):
  - "Permission denied" → suggest `ls -la` to check permissions, `chmod` if needed, or check if the path is a directory that needs `-r` flag.
  - "command not found" → suggest checking if the tool is installed (`which <cmd>`), if it needs `apt-get install`, or if a typo in the command name.
  - "Connection refused" → suggest checking if the service is running, if the port is correct, or if a proxy is needed.
  - "No such file or directory" → this may already be covered; check. If not, suggest `ls` the parent directory and check for typos.
  - "cannot create directory" → suggest checking parent directory permissions and disk space.
  - "syntax error near unexpected token" → suggest checking for unclosed quotes, missing semicolons, or unescaped special characters.
- Read the existing hints first to avoid duplicates. The function likely has 8-12 existing patterns.
- Each hint is a 3-5 line match arm. Keep them focused and actionable.
- Do not refactor the match statement, extract helpers, or change the function signature.
