Title: Add bounded-command detection and timeout-aware recovery hints to bash tool
Files: src/safety.rs, src/tool_wrappers.rs
Issue: none
Origin: planner

Evidence:
- Graph-derived pressure #2: `failed_tool_summary.bash_tool_error=7` — "prefer bounded commands with explicit paths and inspect exit output before retrying broader checks" (trajectory).
- GH Actions log feedback: "command timed out after 120s (3x), command timed out after 180s (2x)" — recurring timeout pattern across sessions.
- Recovery hints improved Day 139 (timing constraints, `./script.sh` prefix, `set -e` advice) but no pre-execution boundedness detection exists.
- Unit test in `tests/integration/bounded_command_detection.rs` should validate the new detection logic.

Edit Surface:
- src/safety.rs (add `check_unbounded_command` function, integrate into `analyze_bash_command`, add tests)
- src/tool_wrappers.rs (optional: wire the new check into tool execution if needed)

Verifier:
- cargo test safety -- --test-threads=1
- cargo test tool_wrappers -- --test-threads=1

Fallback:
- If unbounded-command patterns are too varied for reliable static detection, narrow the check to the most common pattern (recursive grep/find without a starting path, or `find /` with no `-maxdepth`).
- If the safety.rs module test structure makes adding a new check function complex, keep the check minimal (one pattern) and add it inline to `analyze_bash_command`.

Objective:
Reduce bash tool timeout failures by detecting unbounded commands before execution and providing actionable timeout-aware recovery hints when commands do time out.

Why this matters:
Bash tool timeouts waste evolution session time (120s+ per timeout), produce failed tool actions that pollute state/transcript reconciliation, and the current recovery hints don't distinguish between exit-code failures and timeout failures. Adding pre-execution boundedness detection catches the most common failure class (recursive search without bounds) before it runs. Timeout-specific recovery hints help the agent self-correct when a bounded command still times out.

Success Criteria:
- `analyze_bash_command` detects at least these unbounded patterns and returns a warning:
  - `find /` or `find ~` without `-maxdepth`
  - `grep -r` without a starting path that could exhaust the filesystem
  - `rg` without a path argument (searches cwd recursively by default, which is usually fine, but `rg /` would not be)
- Timeout error messages include advice like "try adding --max-depth=N, -m N (max-count), or a specific directory path"
- `cargo test safety` passes with new test cases.
- No false positives on standard bounded commands (e.g., `grep -r pattern src/`, `find src/ -name '*.rs'`).

Verification:
- cargo test safety -- --test-threads=1
- cargo build

Expected Evidence:
- Future trajectory snapshots show fewer `bash_tool_error` occurrences.
- Transcript actions show pre-execution warnings when unbounded commands are attempted.

Implementation Notes:
- Add `check_unbounded_command(cmd: &str) -> Option<String>` to src/safety.rs following the existing pattern of check functions (check_rm_destruction, check_permission_changes, etc.).
- Wire it into `analyze_bash_command` return (append to the warning string if other warnings also exist).
- Patterns to detect:
  - `find` with first non-flag arg being `/`, `~`, `$HOME`, or no path at all, and no `-maxdepth`
  - `grep -r`/`grep --recursive` with first non-flag arg being `/` or `~`
  - `rg` with `/` or `~` as path (rg without a path is fine — it searches cwd)
- Do NOT flag: `find src/ -name '*.rs'`, `grep -r pattern ./src`, `rg pattern` (no path = cwd, bounded), `rg pattern src/`
- For timeout recovery: in src/tool_wrappers.rs `targeted_recovery_hint`, detect timeout error messages (look for "timed out", "timeout", "deadline exceeded") and return specific advice about narrowing the command scope, adding `-maxdepth`, `-m`, or explicit paths.
- Follow existing code style: use `cmd_lower` for case-insensitive matching, check for flag presence before flag absence.
