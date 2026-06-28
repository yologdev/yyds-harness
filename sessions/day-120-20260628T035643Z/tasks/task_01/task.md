Title: Add catch-all pattern to bash targeted recovery hints for unrecognized error output
Files: src/tool_wrappers.rs
Issue: none
Origin: planner

Evidence:
- Trajectory Day 120 graph-derived pressure: `failed_tool_summary.bash_tool_error=3` — "Bound failing shell commands before retrying: prefer bounded commands with explicit paths and inspect exit output before retrying broader checks"
- `targeted_recovery_hint` in `src/tool_wrappers.rs` (line 994-1046) covers 6 specific bash error patterns (exit code, timed out, failed to spawn, no such file or directory, permission denied, command not found) but has no catch-all for unrecognized stderr output. Bash errors that don't match any keyword get no targeted hint at all — the caller gets only the generic `tool_recovery_hint` from `prompt_retry.rs`.
- Corrected log feedback lesson: "shell tool commands failed during the session → prefer bounded commands with explicit paths and inspect exit output before retrying broader checks"
- Assessment: "bash_tool_error=3. Recent sessions show shell command failures. Recovery hints exist but may not be firing effectively."
- This is a single-file, small, testable Rust change — exactly the icebreaker needed to break the 6-session no-code streak.

Edit Surface:
- src/tool_wrappers.rs: the `targeted_recovery_hint` function's bash arm, specifically the final `else { None }` at line 1044. Add a catch-all hint for unrecognized bash error output.

Verifier:
- cargo build && cargo test --lib tool_wrappers

Fallback:
- If the existing tests already cover the catch-all path and no code change is needed, write an obsolete_already_satisfied note explaining that `targeted_recovery_hint` already returns a useful generic message for unrecognized patterns.

Objective:
Every bash tool failure gets a useful targeted recovery hint, even when the error message doesn't match any of the 6 recognized keyword patterns. Close the gap where unrecognized stderr output gets no targeted hint at all.

Why this matters:
The `bash_tool_error=3` signal in the trajectory means implementation agents are hitting bash failures that don't match any recognized pattern, so they get no targeted guidance. The generic `tool_recovery_hint` fires but says "inspect the exit code" — which doesn't help when the error is unrecognized stderr. Adding a catch-all for the bash arm ensures every bash failure gets actionable advice (use explicit paths, break into smaller commands, check $? first).

Success Criteria:
- `targeted_recovery_hint("bash", "some unrecognized error text")` returns `Some(...)` instead of `None`
- The catch-all hint says to check `$?` immediately, use explicit paths, and retry with a simpler bounded command
- Existing tests pass (`cargo test --lib tool_wrappers` — there are tests at line 2933+)
- No behavior change for the 6 already-matched patterns

Verification:
- cargo build
- cargo test --lib tool_wrappers
- cargo test (full suite)

Expected Evidence:
- Future trajectory snapshots show reduced bash_tool_error count (the catch-all prevents unactionable bash failures from cascading)
- The corrected lesson "prefer bounded commands with explicit paths" stops being a recurring recommendation

Implementation Notes:
- Change only the `else { None }` at line 1044 in `targeted_recovery_hint`. The `None` is the gap — unrecognized bash errors get no targeted hint.
- Replace with a `Some(...)` containing a concise catch-all hint: "The shell command failed. Check $? immediately for the exit code, use explicit paths (./script, not script), and retry with a simpler bounded command. Break pipelines into individual steps to isolate the failure."
- The catch-all should be shorter than the pattern-specific hints because it fires for a broader class of errors.
- The function is tested at line 2933+ — add one test case for the catch-all path with an unrecognized error message like "something went wrong".
- No changes to prompt_retry.rs or any other file.
