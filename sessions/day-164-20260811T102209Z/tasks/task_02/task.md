Title: Add targeted bash recovery hints for "Is a directory" and "No space left on device" errors
Files: src/tool_wrappers.rs
Issue: none
Origin: planner

Evidence:
- Day 164 trajectory graph pressure #5: `failed_tool_summary.bash_tool_error=8` — 8 bash commands failed during the latest implementation session.
- The log feedback corrected lesson says: "shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks."
- Assessment: "Shell tool failures (bash_tool_error=8): implementation commands fail during task execution."
- The `targeted_recovery_hint()` function in `src/tool_wrappers.rs` (line 1018) already covers 12+ bash error patterns (exit codes, timed out, failed to spawn, no such file, permission denied, command not found, argument list too long, broken pipe, git fatal errors, network errors) but is missing two common patterns:
  - "Is a directory" — when the agent tries to read/write/execute a path that is a directory, not a file (e.g., `cat some_dir/`, `./some_dir`)
  - "No space left on device" — disk full in CI; actionable but confusing without context
- These are 2-5 line additions each. The pattern match block is a flat `if/else if` chain — adding branches is mechanically simple and carries near-zero regression risk.

Edit Surface:
- src/tool_wrappers.rs

Verifier:
- cargo test tool_wrappers -- --test-threads=1

Fallback:
- If the implementation agent cannot reproduce the exact error messages to test against (needs actual bash failures), add the patterns based on the standard error strings and trust the existing test coverage for the recovery hint dispatch logic (tests at line 2172+).
- If `bash_tool_error` drops to 0 in the next trajectory BEFORE this task is implemented (the problem self-resolved), mark this task obsolete.

Objective:
Add two targeted recovery hints to `targeted_recovery_hint()` in `src/tool_wrappers.rs` for bash error patterns not currently covered: "Is a directory" (EISDIR) and "No space left on device" (ENOSPC).

Why this matters:
`bash_tool_error=8` is the #1 tool failure category in the current trajectory. Implementation agents run broken bash commands, get confused by errors they don't understand, and waste turns retrying without fixing the root cause. Better recovery hints help the agent self-correct on the NEXT attempt instead of getting stuck. Each avoided retry saves a turn that can be used for actual implementation work.

Success Criteria:
- `targeted_recovery_hint("bash", "cat: some_dir: Is a directory")` returns a `Some(String)` with actionable advice (check with `ls -ld`, use `ls` or `find` for directories, or `cd` into the directory first).
- `targeted_recovery_hint("bash", "No space left on device")` returns a `Some(String)` with actionable advice (check disk with `df -h`, clean up with `rm -rf /tmp/*` or `docker system prune`).
- Existing tests (at line 2172+) continue to pass.
- No new clippy warnings.

Verification:
- cargo build
- cargo test tool_wrappers -- --test-threads=1
- cargo clippy --all-targets -- -D warnings 2>&1 | head -5  (confirm no new warnings)

Expected Evidence:
- `failed_tool_summary.bash_tool_error` drops in the next trajectory when implementation agents encounter "Is a directory" or "No space left on device" errors and self-correct instead of getting stuck.
- Task lineage shows a `src/tool_wrappers.rs` change that passed strict verification.

Implementation Notes:
- Add two new `else if` branches in `targeted_recovery_hint()` under the `"bash"` match arm, AFTER the existing `network is unreachable` branch (line 1118) and BEFORE the final `else` (line 1127).
- Pattern 1: match `msg_lower.contains("is a directory")` — the hint should explain that the path is a directory, not a file, and suggest: check with `ls -ld <path>`, use `ls <path>/` to list contents, use `find <path> -type f` to find files, or `cd` into the directory first. Also suggest `file <path>` to confirm the type.
- Pattern 2: match `msg_lower.contains("no space left on device")` — the hint should explain the disk is full, suggest checking with `df -h`, and recommend cleaning up: `rm -rf /tmp/*` (or specific large files), `docker system prune -f` if Docker is in use, or `cargo clean` for Rust target directories.
- Each hint should be 2-4 sentences. Keep the same style as existing hints — direct, actionable, with example commands.
- The change should be under 25 lines total. Do NOT refactor, extract helpers, add tests, or modify any other function.
- Do NOT change the existing `else` fallback (line 1127-1135) — the new patterns go BEFORE it.
