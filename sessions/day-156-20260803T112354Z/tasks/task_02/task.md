Title: Add bounded-command and pipe-safety recovery hints for bash tool failures
Files: src/tool_wrappers.rs
Issue: none
Origin: planner

Evidence:
- Graph-derived next-task pressure #4: "Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=5): prefer bounded commands with explicit paths and inspect exit output before retrying broader checks"
- Corrected log feedback lesson: "shell tool commands failed during the session -> prefer bounded commands with explicit paths and inspect exit output before retrying broader checks"
- The targeted_recovery_hint function in src/tool_wrappers.rs already has bash failure hints but could be more specific about: (a) pipe failures (SIGPIPE / broken pipe) and (b) the general fallback case suggesting bounded commands.
- The current general fallback (line 1096-1104) already mentions bounded limits but buried in a long paragraph — making it more prominent and adding pipe-specific advice would help agents recover faster.

Edit Surface:
- src/tool_wrappers.rs

Verifier:
- cargo test tool_wrappers

Fallback:
- If the existing test coverage for RecoveryHintTool already covers all hint paths and the change is purely textual, verify with cargo test and mark complete.
- If the implementation agent can't find a meaningful improvement beyond what's already in the hints, document that and mark the task as no-op (the hints are already good).

Objective:
Improve bash tool recovery hints in targeted_recovery_hint() to provide more specific advice for pipe failures and to make bounded-command advice more prominent in the general failure case.

Why this matters:
Bash tool errors waste agent turns on retries that repeat the same unbounded command. Better recovery hints reduce the retry loop by giving the agent the right fix on the first failure. The trajectory shows 5 bash tool errors across recent sessions — each one that leads to a better retry saves a full turn (~30-60 seconds of wall clock time and token cost).

Success Criteria:
- The broken pipe hint (already exists at line 1089-1095) is extended to mention `head -n N` as the common cause — when piping into `head`, the writer gets SIGPIPE when head closes after N lines. This is normal and the fix is to use `tail` or process differently.
- The general fallback hint (line 1096-1104) is reorganized to put bounded-command advice first: "Start with a bounded version: add `| head -n 20` or `--max-results 5` to test the command shape before running unbounded."
- All existing tests in src/tool_wrappers.rs continue to pass.
- The change is purely textual (hint strings only) — no logic changes.

Verification:
- cargo test tool_wrappers
- cargo build

Expected Evidence:
- Future trajectory shows lower bash_tool_error count as agents receive better recovery hints.
- No regression in existing hint tests.

Implementation Notes:
- Edit only the hint strings in targeted_recovery_hint() — do not change function signatures, control flow, or add new match arms.
- The broken pipe hint (around line 1089) can mention: "This often happens when piping into `head -n N` — head closes after N lines and the writer gets SIGPIPE. Use `tail`, `sed -n`, or redirect to a file instead."
- The general fallback (around line 1096) should lead with bounded-command advice before mentioning exit codes or paths.
- Keep changes minimal — this is a 20-minute task.
