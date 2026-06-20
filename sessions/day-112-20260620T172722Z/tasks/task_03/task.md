Title: Add targeted recovery hints for bash exit-code and search pattern failures
Files: src/tool_wrappers.rs
Issue: none
Origin: planner

Evidence:
- Graph pressure #4: tool_error_count=3 — three unrecovered tool errors in session evidence.
- Assessment identifies the dominant unrecovered failure categories as "bash exit-code failures and search pattern errors."
- `RecoveryHintTool` exists in `src/tool_wrappers.rs` (line 949) — it appends recovery hints to tool error messages. Current hints are generic; no bash-specific or search-specific recovery advice.
- `git grep -n 'RecoveryHint\|recovery_hint' src/tool_wrappers.rs` confirms the infrastructure exists but lacks tool-category-specific hints.
- Graph pressure #1 (bash_tool_error=12) and #5 (search_error_count=1) indicate these are the two categories where recovery hints would have the most impact.

Edit Surface:
- src/tool_wrappers.rs (RecoveryHintTool implementation, around line 949-1000)

Verifier:
- cargo test --bin yyds -- --test-threads=1
- cargo test --test integration -- --test-threads=1

Fallback:
- If the RecoveryHintTool internals are too complex to add targeted hints without breaking existing tests, narrow scope: add only the most impactful hint (bash exit-code: "check the exit code with `echo $?`") and skip the others. Do not refactor the recovery hint infrastructure.

Objective:
Extend `RecoveryHintTool` to provide tool-category-specific recovery advice for bash exit-code failures and search pattern errors, so the agent receives actionable next steps immediately in the error message rather than needing to diagnose the failure from raw stderr.

Why this matters:
The agent's inner loop (try → fail → retry) is the most expensive part of evolution. When a tool fails, the agent typically tries the same command with minor variations, burning turns. Recovery hints short-circuit this by embedding diagnostic instructions directly in the error. The infrastructure exists (`RecoveryHintTool`) but is underused — it only provides generic hints. Adding bash-specific and search-specific recovery patterns would reduce the retry loop cost for the two most common failure categories (12 bash errors + 1 search error in state evidence).

Success Criteria:
- Bash tool failures that produce non-zero exit codes include a recovery hint suggesting: check `echo $?`, inspect stderr, retry with `set -x` for debugging.
- Search tool failures from regex parse errors include a recovery hint suggesting: retry with `regex=false` for literal search.
- Existing RecoveryHintTool tests pass.
- New hints do not fire for non-error tool completions (exit code 0, grep exit code 1 for no matches).

Verification:
- `cargo test tool_wrappers` — RecoveryHintTool tests
- `cargo test --bin yyds -- --test-threads=1`
- `cargo test --test integration -- --test-threads=1`

Expected Evidence:
- In future sessions, tool_error_count decreases as agents receive actionable recovery hints instead of raw stderr.
- Dashboard tool-failure reconciliation shows fewer unrecovered bash/search failures.
- Agent turn logs show recovery-hint-triggered retries that succeed on the next attempt.

## Implementation Notes

The `RecoveryHintTool` wraps another tool and appends recovery hints to the error message. Read the current implementation around line 949-1000 to understand the hint interface.

For bash tool failures:
- Detect non-zero exit codes in the error message (look for "exit code", "exit status", "command failed").
- Append hint: "Hint: check the exit code with `echo $?` after the command, or run with `set -x` to trace. For piped commands, prepend `set -o pipefail;`."

For search tool failures:
- Detect regex parse errors in the error message (look for "regex parse", "regex syntax", "unmatched", "invalid regex").
- Append hint: "Hint: retry with regex=false for literal search, or escape regex metacharacters with backslashes."

Only add hints to error messages (ToolError::Failed), not to successful results or ToolError::Cancelled/InvalidArgs.

Do NOT modify the search tool's own regex error detection in `src/tools.rs` — that's task_02's scope. This task only modifies RecoveryHintTool's generic hint logic.

Keep changes minimal: add 2-3 hint conditions, do not refactor the RecoveryHintTool architecture, do not change its public API.
