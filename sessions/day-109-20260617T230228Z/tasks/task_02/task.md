Title: Extend path-finding recovery hints to search, edit_file, and bash tools
Files: src/prompt_retry.rs
Issue: none
Origin: planner

Evidence:
- Assessment found: "Path discovery on tool failure: The read_file recovery hint improvement (Day 109 18:19) is recent but only covers one tool. search, edit_file, and bash could benefit from similar 'here's how to find the right thing' recovery hints."
- Trajectory graph-derived pressure: "Verify readable paths before file reads (failed_tool_summary.read_error=2): verify paths with rg --files and prefer module or symbol discovery"
- Current code in src/prompt_retry.rs: read_file attempt-1 hint (lines 151-155) has concrete path-finding commands (`rg --files | grep <name>`, `list_files`); search (line 157), edit_file (line 143-145), bash (lines 138-141) have only generic fallback text without concrete path-discovery commands.

Edit Surface:
- src/prompt_retry.rs

Verifier:
- cargo check && cargo test prompt_retry

Fallback:
- If the read_file hint was already reverted or no longer follows the same pattern, mark this task obsolete rather than inventing a new pattern.

Objective:
Give search, edit_file, and bash the same level of concrete path-finding recovery hints that read_file already has, so the agent can self-correct path errors across all file-access tools.

Why this matters:
The assessment identified that the read_file recovery hint was improved in the Day 109 18:19 session with specific commands for path discovery. But search, edit_file, and bash still have generic fallback text ("The search failed. Try a simpler pattern or check the path."). When the agent hits a path-not-found error with these tools, it gets no actionable guidance. This directly addresses the trajectory's "verify readable paths before file reads" pressure by making all file-access tools give the agent the same path-discovery instructions.

Success Criteria:
- search attempt-1 recovery hint includes concrete path-finding commands (e.g., `rg --files | grep <name>`)
- edit_file attempt-1 recovery hint includes path verification suggestion (e.g., verify the file path exists before retrying)
- bash attempt-1 recovery hint references path-checking commands (e.g., `test -f`, `ls`)
- All existing tool_recovery_hint match arms still compile
- No behavior change for attempt >= 2 escalation hints

Verification:
- cargo check
- cargo test prompt_retry
- cargo test -- --test-threads=1 (full suite to ensure no regressions)

Expected Evidence:
- Future sessions where search/edit_file/bash fail on path-not-found will show the agent using the concrete recovery commands to find the correct path.
- read_error count in trajectory should decrease as all file-access tools now give path-finding guidance.

Implementation Notes:
- Keep changes ONLY in the `tool_recovery_hint` function (lines 100-162 in src/prompt_retry.rs).
- Follow the read_file pattern: suggest `rg --files | grep <name>` for filename discovery, `list_files` for directory exploration, and `rg -n '<symbol>' src/` for symbol-to-file mapping.
- For search: the most common failure is a non-existent path argument — suggest verifying the directory exists with `ls` or `list_files`.
- For edit_file: the most common failure is old_text mismatch — keep the existing read_file suggestion but add path verification.
- For bash: keep the existing exit-code/stderr guidance but add explicit path-checking commands like `test -f <path>`, `ls <dir>`, `rg --files | head`.
- Do NOT change any match arm for attempt >= 2 (the escalation hints at lines 101-133).
