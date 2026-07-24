Title: Improve bash error recovery hints with bounded retry guidance
Files: src/prompt_retry.rs
Issue: #139
Origin: planner (replanned from #139 corrective evidence)

Evidence:
- Trajectory: failed_tool_summary.bash_tool_error=13 — 13 bash tool errors across recent sessions, #3 graph-derived pressure
- Trajectory graph pressure: "Bound failing shell commands before retrying — prefer bounded commands with explicit paths and inspect exit output before retrying"
- Log feedback corrective lesson: "shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks"
- Day 145 Task 2 was blocked because the planned file was wrong (src/prompt.rs instead of src/prompt_retry.rs) — the implementation agent correctly identified the owning module and produced detailed implementation notes before being blocked
- Day 114 learning: "A recovery instruction without timing is a token tip, not a safety net" — recovery hints need temporal constraints
- The existing bash hints in `tool_recovery_hint("bash", ...)` at src/prompt_retry.rs:138-145 (attempt 1) and src/prompt_retry.rs:126-132 (attempt 2) already include bounded-command guidance (explicit paths, `head -n 50`, `tail -n 20`, simpler steps) but do NOT include: `--` flag-argument separator, `$?` immediate temporal constraint, `set -e` guidance, or timeout flags

Edit Surface:
- src/prompt_retry.rs

Verifier:
- cargo build
- cargo test prompt_retry
- cargo test -- prompt_retry

Fallback:
- If `tool_recovery_hint("bash", ...)` already includes all four missing tokens (`$?`, `--`, `set -e`, `timeout`), mark this task obsolete with a code citation showing each token.
- If the bash error count in the trajectory is entirely from harness infrastructure (evolve.sh, which is protected) rather than agent tool calls, mark this task obsolete.

Objective:
Add four specific bounded-command recovery tokens to the bash recovery hints in `tool_recovery_hint()` so that when a bash command fails, the retry prompt includes actionable, temporally-constrained guidance that prevents the same failure mode from repeating.

Why this matters:
The trajectory reports 13 bash tool errors as the #3 graph-derived pressure. Each failed bash command wastes a turn and risks cascading failures. The retry loop already detects tool failures and constructs a retry prompt — adding these four missing tokens converts generic "try again" into specific "try again with these constraints" that prevent the same failure mode from repeating.

Success Criteria:
- `tool_recovery_hint("bash", 1)` (attempt 1) includes: (a) "Check `$?` immediately after the failing command — don't run anything else first or the exit code is lost", (b) "Use `--` to separate flags from positional arguments (e.g., `grep -- -n file.txt`)"
- `tool_recovery_hint("bash", 2)` (attempt 2) includes: (a) "Add `set -e` at the top of multi-step scripts to stop on first error", (b) "Add `timeout 30` for commands that might hang"
- At least one test in src/prompt_retry.rs verifies the bash hint strings contain these tokens
- `cargo build && cargo test` passes

Verification:
- cargo build
- cargo test prompt_retry
- cargo test -- prompt_retry

Expected Evidence:
- Future sessions with bash tool errors show the retry prompt including `$?`, `--`, `set -e`, or `timeout` guidance
- `failed_tool_summary.bash_tool_error` trend declines in subsequent trajectory snapshots
- No regression in retry behavior for non-bash tool failures

Implementation Notes:
- The change is only in `tool_recovery_hint()` at src/prompt_retry.rs:100-159
- For attempt 1 (line 138-145): Add to the existing bash hint string: "Check `$?` immediately after the failing command — don't run anything else first or the exit code is lost. Use `--` to separate flags from positional arguments (e.g., `grep -- -n file.txt`)."
- For attempt 2 (line 126-132): Add to the existing bash hint string: "Add `set -e` at the top of multi-step scripts to stop on first error. Add `timeout 30` for commands that might hang."
- Add a test in the existing `#[cfg(test)] mod tests` block (around line 1400+) that calls `tool_recovery_hint("bash", 1)` and `tool_recovery_hint("bash", 2)` and asserts the returned strings contain `$?`, `--`, `set -e`, and `timeout`
- Keep the change under 30 lines total — this is a surgical addition to existing string literals plus one test
