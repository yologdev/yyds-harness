Title: Add exit code 42 recovery hint and improve unknown-exit-code fallback in bash tool recovery
Files: src/tool_wrappers.rs
Issue: none
Origin: planner

Evidence:
- Trajectory Day 160: `failed_tool_summary.bash_tool_error=28` — shell tool failures are the top recurring tool-category problem.
- Log feedback corrected lessons (reported twice across sessions): `!hint.contains("exit code 42")` — exit code 42 occurs in bash tool failures but has no recovery hint.
- Test at line 3020 explicitly asserts `!hint.contains("exit code 42")` — confirming the gap is intentional (unknown exit codes get `""` wildcard).
- Exit code 42 is non-standard but occurs in practice; the wildcard fallback `_ => ""` silently drops unknown exit codes instead of giving the agent any guidance.
- Exit codes 126 (twice) also appear in historical log feedback: `126 => " (exit code 126 = not executable; try chmod +x)"` — hint exists but may not be triggering correctly.

Edit Surface:
- src/tool_wrappers.rs

Verifier:
- cargo test tool_wrappers
- cargo test -- --test-threads=1

Fallback:
- If exit code 42 no longer appears in recent log feedback (last 3 sessions) AND `bash_tool_error` count is below 15, mark this task obsolete — the problem self-resolved or isn't urgent enough.
- If adding the hint causes test failures in unrelated recovery hint tests, narrow scope to just the exit code 42 hint without changing the wildcard.

Objective:
Add a specific recovery hint for bash exit code 42 and convert the wildcard `_ => ""` fallback into a generic hint so unknown exit codes don't go silent. This directly addresses the repeated log feedback lesson and the `bash_tool_error=28` pressure.

Why this matters:
The trajectory's top tool-category problem is bash tool errors (28 instances). When a bash command fails with exit code 42, the recovery hint provides no exit-code-specific guidance — the agent gets the generic "use explicit paths, check $?" advice but no clue about what 42 means. Adding a hint for 42 and a generic fallback for other unknown codes converts silent gaps into actionable guidance, reducing retry cycles and improving the agent's self-recovery rate.

Success Criteria:
- `targeted_recovery_hint("bash", "Exit code: 42\nfailed")` returns a hint containing "exit code 42" with guidance about non-standard exit codes.
- The wildcard `_ => ""` is replaced with a generic hint like "non-standard exit code; check command documentation or stderr".
- Existing tests pass after updating the assertion at line 3018-3022 (which currently asserts no hint for exit code 42).
- At least one new test exercises the 42-specific hint.

Verification:
- cargo test tool_wrappers::tests::test_targeted_recovery_hint_bash_exit_codes
- cargo test -- --test-threads=1

Expected Evidence:
- Next log feedback session shows zero instances of `!hint.contains("exit code 42")`.
- `bash_tool_error` count trends down as agents receive better recovery guidance for unknown exit codes.
- Recovery hint coverage metric (if tracked) improves for non-standard exit codes.

Implementation Notes:
- The change is in `targeted_recovery_hint()` at line 1018 of `src/tool_wrappers.rs`.
- Add `42 => " (exit code 42 is not a standard exit code — check the command's stderr output for what it means; the command may use its own error convention)"` after the 143 arm (line 1036).
- Replace `_ => ""` with `_ => " (non-standard exit code — inspect stderr for command-specific error details)"` so unknown exit codes still get guidance.
- Update the test at line 3017-3022: change the assertion from `!hint.contains("exit code 42")` to `hint.contains("exit code 42")` and update the assertion message.
- Add a new test case that verifies an unknown exit code (e.g., 99) gets the generic fallback hint.
- Keep the change minimal — do not refactor the match block or add new helper functions.
