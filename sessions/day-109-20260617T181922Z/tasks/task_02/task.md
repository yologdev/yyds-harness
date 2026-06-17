Title: Improve read_file recovery hints with specific path-finding commands
Files: src/prompt_retry.rs
Issue: none
Origin: planner

Evidence:
- Trajectory graph pressure row "Verify readable paths before file reads (read_error=2)": file reads that hit path errors in recent sessions. The corrected log_feedback lesson: "verify paths with rg --files and prefer module or symbol discovery when exact files are uncertain."
- Trajectory corrected top lessons: "file-read evidence contained path or access errors -> verify paths with rg --files and prefer module or symbol discovery when exact files are uncertain"
- Log feedback score 0.9688 — the only active friction signal dragging the score below 1.0 is file-read path errors.
- Existing RecoveryHintTool (`src/tool_wrappers.rs:949`) already intercepts read_file failures and calls `prompt_retry::tool_recovery_hint("read_file", attempt)` to append hints. The current hint at attempt 1 is: "The file read failed. Use list_files to verify the path, or search for the file." This is generic and doesn't tell the agent HOW to find the right path.
- The yoagent read_file built-in returns `ToolError::Failed("File not found: <path>")` on missing files, which the RecoveryHintTool wraps with the hint.

Edit Surface:
- src/prompt_retry.rs: enhance the `tool_recovery_hint` function's `read_file` hints at both attempt 1 and attempt 2 to include concrete, copy-pasteable commands that help the agent find the correct path.

Verifier:
- cargo test prompt_retry
- cargo check

Fallback:
- If the read_file error message format changed in a newer yoagent release and the hint no longer makes sense, note the change but still improve the hint text.
- Do not add filesystem access to the RecoveryHintTool — this is a textual hint improvement only.

Objective:
Make the recovery hints for failed `read_file` calls concrete enough that the agent can immediately run a command to find the correct path, instead of guessing or trying broad searches.

Why this matters:
When the agent reads a file that doesn't exist, it currently gets: "The file read failed. Use list_files to verify the path, or search for the file." This is a suggestion of tools to try, not a command to run. The agent then often tries a second incorrect path, wastes turns, and risks the same error. A concrete hint like "Try: rg --files | grep src/commands_state" converts the suggestion into an immediate action that has a high probability of finding the right path on the first retry.

Success Criteria:
- Attempt 1 hint for read_file: includes a concrete command template using `rg --files` or `ls` that the agent can adapt with the filename it was looking for
- Attempt 2 (escalation) hint for read_file: suggests the bash-based fallback (`cat`, `head`) AND a broader path-finding strategy
- Existing tests in `src/prompt_retry.rs` continue to pass (tests check for substring "read_file" in hints)
- The hint is concrete without being hallucinatory — it should not suggest a specific file path, only a command pattern

Verification:
- cargo test prompt_retry
- cargo test -- tool_wrappers (tests that reference tool_recovery_hint for read_file)
- Manual check: `cargo run -- state tail --limit 1` to confirm no regressions

Expected Evidence:
- Future log_feedback scores should show reduced read_error counts
- Agent transcripts should show fewer "file not found" retries after the first failure
- The trajectory's corrected lesson "verify paths with rg --files" becomes embedded in the tool hint, not just in documentation

Implementation Notes:
- Only modify the `tool_recovery_hint` function in `src/prompt_retry.rs` (lines 100-158).
- For attempt 1 (line 149): change from "The file read failed. Use list_files to verify the path, or search for the file." to something like:
  "The file read failed — the path doesn't exist. Verify the correct path: run `rg --files | grep <name>` (replace <name> with the filename you were looking for), or use `list_files` on the parent directory to see what's actually there."
- For attempt 2 (line 108): change from "Try bash instead: use `cat <path>` or `head -n 100 <path>` to read the file contents directly." to something like:
  "Still failing. Run `rg --files` to list all tracked files and find the exact path, then use `cat <exact-path>` or `head -n 100 <exact-path>` to read it. Or use `rg -n '<symbol>' src/` to find which file defines the symbol you need."
- Keep the hint text concise — 2-3 sentences max. The goal is actionable, not verbose.
- The function is `pub` — check if any other crate or test references it to ensure compatibility. Use `rg 'tool_recovery_hint' src/` before finalizing.
