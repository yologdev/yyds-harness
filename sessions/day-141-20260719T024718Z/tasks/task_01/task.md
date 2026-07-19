Title: Add bounded-command pre-execution detection to bash safety checker
Files: src/safety.rs
Issue: #119 (scoped down — safety.rs only, drop tool_wrappers.rs)
Origin: planner

Evidence:
- Graph-derived pressure #4: `failed_tool_summary.bash_tool_error=7` — the dominant tool failure category across multiple sessions (trajectory 2026-07-19T02:51Z).
- GH Actions log feedback: "command timed out after 120s (3x), command timed out after 180s (2x)" — recurring timeout pattern (assessment).
- Log feedback corrected lesson: "shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks."
- The previous attempt (#119) was reverted twice because it was too broad (touching both `src/safety.rs` AND `src/tool_wrappers.rs`). This task scopes down to only the pre-execution detection in `src/safety.rs`.
- `src/safety.rs` already has 20+ check functions following the pattern `fn check_X(cmd: &str) -> Option<String>` wired into `analyze_bash_command` at line 23. Adding one more check function is low-risk.

Edit Surface:
- src/safety.rs (add `check_unbounded_command` function, wire into `analyze_bash_command`, add test module cases)

Verifier:
- cargo test safety -- --test-threads=1

Fallback:
- If unbounded-command patterns are too varied for reliable static detection, narrow to the single most common pattern: `find /` or `find ~` without `-maxdepth`.
- If the function signature or test structure differs from existing check functions, match the simplest check function in the file (e.g., `check_rm_destruction` at line 191).
- If `analyze_bash_command` wiring is unclear, add the call after the last existing check function call.

Objective:
Reduce bash tool timeout failures by detecting unbounded commands before execution and returning a warning that helps the agent self-correct.

Why this matters:
Bash tool timeouts waste evolution session time (120s+ per timeout), produce failed tool actions that pollute state/transcript reconciliation, and are the single most frequent tool failure category (bash_tool_error=7). The timeout recovery hint in `src/tool_wrappers.rs` already exists (line 1048) but only fires after the timeout — pre-execution detection catches the most common failure class (recursive search without bounds) before it runs.

Success Criteria:
- `check_unbounded_command` detects at least these patterns and returns a warning:
  - `find /` or `find ~` without `-maxdepth` flag
  - `grep -r /` or `grep -r ~` (recursive grep from root/home)
  - `rg /` or `rg ~` (ripgrep from root/home)
- Warnings suggest adding `-maxdepth N`, `--max-depth N`, or a specific directory path
- No false positives on bounded commands (e.g., `find src/ -name '*.rs'`, `grep -r pattern ./src`, `rg pattern src/`)
- `cargo test safety -- --test-threads=1` passes with new test cases

Verification:
- cargo test safety -- --test-threads=1
- cargo build

Expected Evidence:
- Future trajectory snapshots show fewer `bash_tool_error` occurrences.
- Transcript actions show pre-execution warnings when unbounded commands are attempted.
- `cargo test safety` includes test cases for bounded and unbounded command patterns.

Implementation Notes:
- Add `fn check_unbounded_command(cmd: &str) -> Option<String>` following the existing pattern (see `check_rm_destruction` at line 191, `check_git_force` at line 235).
- Wire into `analyze_bash_command` (line 23): add a call like `if let Some(w) = check_unbounded_command(command) { warnings.push(w); }` after the last existing check call.
- Patterns to detect (use `cmd_lower` for case-insensitive matching):
  - `find` with first non-flag arg being `/`, `~`, `$HOME`, or no path, and no `-maxdepth` flag present
  - `grep -r` or `grep --recursive` with path arg being `/` or `~`
  - `rg` with path arg being `/` or `~` (rg without a path is fine — it searches cwd)
- Do NOT flag: `find src/ -name '*.rs'`, `grep -r pattern ./src`, `rg pattern` (no path = cwd), `rg pattern src/`
- Keep the change under 60 lines (function + wiring + tests). If it grows beyond that, reduce the pattern set.
