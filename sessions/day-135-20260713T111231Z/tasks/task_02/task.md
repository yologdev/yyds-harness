Title: Add bounded-command and path-verification recovery hints to bash tool failures
Files: src/tool_wrappers.rs src/prompt_retry.rs
Issue: none
Origin: planner (from log feedback corrected lessons)

Evidence:
- Log feedback corrected top lessons for next run (Day 135 trajectory, score=0.4825):
  1. "shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks"
  2. "agent read or searched paths that did not exist -> verify guessed paths with rg --files before reading them, then search owning symbols instead of retrying absent paths"
- Day 135 (04:55) session: reverted_unlanded_source_edits=1, reverted_unverified=1 — implementation agent may have hit these failure modes.
- RecoveryHintTool already exists in src/tool_wrappers.rs and tool_recovery_hint in src/prompt_retry.rs — these are the right insertion points for new hints.

Edit Surface:
- src/tool_wrappers.rs (RecoveryHintTool)
- src/prompt_retry.rs (tool_recovery_hint)

Verifier:
- cargo build && cargo test -- --test-threads=1

Fallback:
- If RecoveryHintTool and tool_recovery_hint already cover bounded-command and path-verification guidance, mark this task obsolete with exact line references.

Objective:
Add two new recovery hints to the bash tool failure path:
1. When a bash command fails: suggest using explicit paths (./script.sh), bounded commands with specific flags, and checking $? immediately before running follow-up commands.
2. When a read_file or search fails with "file not found" or empty results: suggest verifying the path with `rg --files | grep <pattern>` before retrying, and searching for the owning module/symbol instead of retrying the absent path.

Why this matters:
These are recurring failure patterns detected across sessions by the log feedback system. When the implementation agent hits these failures during a task, the recovery hints help it self-correct without burning turns on blind retries. This directly addresses task_unlanded_source_count and task_analysis_only_attempt_count by reducing wasted implementation turns.

Success Criteria:
- RecoveryHintTool for bash failures includes hints about explicit paths, bounded commands, and immediate $? inspection.
- RecoveryHintTool for read_file/search failures includes hints about path verification with rg --files and fallback to symbol search.
- Existing recovery hint tests still pass.

Verification:
- cargo build
- cargo test -- --test-threads=1
- cargo test src/prompt_retry.rs (or the specific test names for tool_recovery_hint)

Expected Evidence:
- Log feedback corrected lessons no longer list "shell tool commands failed" and "agent read or searched paths that did not exist" as top lessons after 3+ sessions.
- task_success_rate improves (fewer turns wasted on blind retries).

Implementation Notes:
- tool_recovery_hint in src/prompt_retry.rs takes a tool name and error code and returns a hint string. Add cases for "bash" and "read_file"/"search" tool names.
- RecoveryHintTool in src/tool_wrappers.rs wraps a tool and appends hints on failure. Check if the hint insertion point already handles these tool names.
- Keep hints concise (1-2 lines). They're appended to the error message the model sees.
- The existing hint for "bash" may already exist — check first. If it does, refine it to include the bounded-command and path guidance.
- Do NOT modify src/safety.rs unless the bash safety analysis is the right insertion point (but RecoveryHintTool is the more targeted location).
