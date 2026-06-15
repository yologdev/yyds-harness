Title: Improve bash retry hints — bounded commands, exit inspection, explicit paths
Files: src/prompt_retry.rs
Issue: none
Origin: planner

Objective:
Enhance the `tool_recovery_hint` function's bash-specific guidance so retry prompts tell the agent to: (1) inspect the command's exit code and error output, (2) prefer explicit absolute paths, and (3) avoid unbounded or recursive commands. This reduces retry churn from bash failures that could be diagnosed without retrying.

Why this matters:
Graph pressure #2: `failed_tool_summary.bash_tool_error=2` with the suggestion "prefer bounded commands with explicit paths and inspect exit output before retrying." The current bash recovery hints are generic ("check if command exists, try simpler version") and don't guide the agent toward the specific failure-analysis steps that prevent repeated identical failures.

Success Criteria:
- Bash retry hint at attempt 1 includes a suggestion to inspect exit code and error output
- Bash retry hint at attempt 2+ includes guidance to use explicit paths and avoid unbounded commands
- Existing hints for other tools are unchanged
- build_retry_prompt and build_auto_retry_prompt continue to work correctly

Verification:
- cargo test prompt_retry
- cargo test -- --test-threads=1
- cargo build

Expected Evidence:
- Future sessions: bash retry prompts show the new guidance text
- Graph pressure #2: failed_tool_summary.bash_tool_error frequency should decrease
- Audit log: bash retry sequences show more diagnostic first steps before retrying

Implementation Notes:

Edit `tool_recovery_hint` in `src/prompt_retry.rs` (lines 100-155).

Change the attempt-1 bash hint from:
```
"The shell command failed. Check if the command exists, \
 try a simpler version, or use a different approach."
```
to something like:
```
"The shell command failed. Inspect the exit code and stderr output above \
 to understand why. Check if the command and any file paths exist, \
 try a simpler bounded version, or use a different approach."
```

Change the attempt-2+ bash hint from:
```
"Try a simpler command: break the command into smaller steps, \
 check if the binary exists with `which <cmd>`, or try an alternative \
 tool (e.g., read_file instead of cat, search instead of grep)."
```
to something like:
```
"Try a simpler bounded command: break into smaller steps with explicit \
 absolute paths. Check exit output first — don't retry the same command \
 without understanding the failure. Verify paths exist with `ls` or `test -f`. \
 Prefer targeted tools (read_file, search) over complex shell pipelines. \
 Avoid unbounded/recursive commands like `rm -rf`, `find /`, or unconstrained globs."
```

Keep both hints concise — they're injected into retry prompts that already include the error summary.
