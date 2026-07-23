# Task 02 Blocked — File Mismatch

**Task**: Improve bash error recovery hints with bounded retry guidance  
**Planned Files**: `src/prompt.rs`  
**Actual Owner**: `src/prompt_retry.rs`

## Evidence

The task's `Edit Surface` and `Files` list names `src/prompt.rs`. However, the
retry prompt construction and recovery hints live in `src/prompt_retry.rs`:

- `build_auto_retry_prompt` (line 64) — constructs the retry prompt text
- `tool_recovery_hint` (line 100) — returns tool-specific recovery hints,
  including the existing bash attempt-1 and attempt-2 hints

The functions in `src/prompt.rs` (`run_prompt_auto_retry` at line 1405,
`run_prompt_auto_retry_with_content` at line 1462) only call
`build_auto_retry_prompt` with the tool name. They contain no recovery hint
text and are not the right place to add bash-specific bounded-command
guidance.

The existing bash recovery hints already include:
- **Attempt 1**: exit code inspection, path verification (`test -f`, `ls`, `rg --files`),
  bounded output (`head -n 50`, `tail -n 20`)
- **Attempt 2**: explicit absolute paths, bounded steps, path verification,
  unbounded-command avoidance

What's not yet covered (per the task's Implementation Notes):
- `--` separator for flags vs positional args
- `$?` immediate temporal constraint ("check $? immediately after the failing command")
- `set -e` guidance
- timeout flags

## Corrected Files

For a future task:
```
Files: src/prompt_retry.rs
```

## What Would Be Needed

A future task targeting `src/prompt_retry.rs` would:
1. Enhance `tool_recovery_hint("bash", 1)` to include: "Check `$?` immediately
   after the failing command — don't run anything else first or the exit code is
   lost. Use `--` to separate flags from positional arguments (e.g.,
   `grep -- -n file.txt`)."
2. Enhance `tool_recovery_hint("bash", 2)` to include: "Add `set -e` at the top
   of multi-step scripts to stop on first error. Add `timeout 30` for commands
   that might hang."
3. Add a test in `src/prompt_retry.rs` verifying the bash hint includes
   bounded-command tokens like `$?`, `--`, or `set -e`.

TASK_TERMINAL_EVIDENCE: blocked
