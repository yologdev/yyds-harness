Title: Enhance bash recovery hints with path-bounding and exit-code inspection guidance
Files: src/tool_wrappers.rs
Issue: none
Origin: planner

Evidence:
- Graph-derived pressure #1: "Bound failing shell commands before retrying" with `failed_tool_summary.bash_tool_error=6`.
- Recommendation from trajectory: "prefer bounded commands with explicit paths and inspect exit output before retrying."
- The RecoveryHintTool in `src/tool_wrappers.rs` already detects bash exit-code failures (line ~991: "exit code", "exit status", "command failed") but only suggests a generic `echo $?` check.
- The tool already has pattern-specific hints for "timed out", "failed to spawn", "no such file", "permission denied", and "command not found" — the exit-code path is the weakest link.
- Bash tool errors in the retry loop waste tokens because the agent often retries the same failing command without understanding what went wrong.

Edit Surface:
- src/tool_wrappers.rs — `targeted_recovery_hint` function (the bash exit-code arm) and any related test functions

Verifier:
- cargo test --lib tool_wrappers -- --test-threads=1
- cargo build

Fallback:
- If `grep -c 'exit code\|exit status\|command failed' src/tool_wrappers.rs` shows the hint already includes path qualification and exit-code inspection, write an obsolete-task note instead of editing. Check: the hint should mention both explicit paths AND exit-code inspection beyond just `echo $?`.

Objective:
Make the RecoveryHintTool's bash exit-code hint actionably preventive: when a bash command fails with an exit code, the recovery hint should guide the agent toward explicit paths, bounded commands, and concrete exit-code inspection before retrying the same command.

Why this matters:
Bash is the most-used tool (3,925 invocations). Each time a bash command fails and the agent retries without understanding the failure, it wastes tokens and adds noise to tool-failure metrics. A better recovery hint turns the retry from a blind reattempt into an informed fix. This directly addresses the recurring `failed_tool_summary.bash_tool_error` signal in graph-derived pressure.

Success Criteria:
- The bash exit-code recovery hint mentions explicit paths (e.g., "use `./script.sh` instead of `script.sh`") as a diagnostic step.
- The hint includes exit-code inspection guidance beyond generic `echo $?` — e.g., checking `$?` immediately after the specific failing command, or using `set -e` / `set -o pipefail` for multi-command scripts.
- Existing tests for `targeted_recovery_hint` still pass (the hint text changes but the structure doesn't).
- At least one new test assertion verifies the enhanced hint text includes path-bounding language.

Verification:
- cargo test --lib tool_wrappers -- --test-threads=1
- cargo build
- cargo fmt --check

Expected Evidence:
- Future sessions show fewer `failed_tool_summary.bash_tool_error` pressure signals (< 3 in the next 5 sessions, down from current 6).
- Graph-derived pressure row "Bound failing shell commands before retrying" drops from the top-4 pressure list within 3 sessions.
- Tool-failure transcripts show agents applying the path-bounding advice before retrying bash commands.

Implementation Notes:
- The change is surgical: modify only the string returned in the `"bash"` arm of `targeted_recovery_hint` for the exit-code/exit-status/command-failed branch (line ~991-1003 in `src/tool_wrappers.rs`).
- Add 1-2 test assertions in the existing `test_targeted_recovery_hint_bash_exit_code` test (line ~2868) or nearby test functions that verify the new hint text.
- Keep the hint concise — 2-3 sentences max. The RecoveryHintTool appends to the error message, not replaces it.
- The hint should mention: (1) use explicit paths (./script.sh), (2) check `$?` immediately after the failing command in the original multi-command sequence, (3) consider adding `set -e` to fail fast on any error.
- Do not change the matching logic (which error strings trigger this hint) — only change the hint text and add test coverage.
- Do not read unrelated source files. The task is scoped to one file.
