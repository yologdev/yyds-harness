Title: Suggest rg --files in read_file "no such file" recovery hint
Files: src/tool_wrappers.rs
Issue: none
Origin: planner

Evidence:
- Log feedback top corrected lesson: "agent read or searched paths that did not exist -> verify guessed paths with rg --files before reading them, then search owning symbols instead of retrying absent paths." This lesson has appeared across multiple sessions and hasn't been encoded in the tool recovery path.
- The current `targeted_recovery_hint` for `read_file` when the file doesn't exist (line 1123-1129) says: "Use `list_files` to discover the correct path, or check for typos in the path you provided." But `list_files` can be slow on large repos; `rg --files` (from ripgrep) is a faster, more precise alternative that's already installed and used throughout the codebase.
- The system prompt's "Bounded Context Use" section already says: "Verify candidate paths with repo file listing before reading/searching guessed files; if a path is absent, search for the owning module or symbol instead of retrying the missing path." The tool hint should reinforce this by mentioning both `rg --files` (fast verification) and searching for the owning symbol (fallback to avoid retry loops).

Edit Surface:
- src/tool_wrappers.rs — update the `"read_file"` branch of `targeted_recovery_hint` for the "no such file or directory" case

Verifier:
- cargo test tool_wrappers -- --test-threads=1
- Specifically: `test_targeted_recovery_hint_read_file_no_such_file` must pass

Fallback:
- If the existing test doesn't match the new hint text, update the test assertion to match. If the hint function signature or surrounding code has changed significantly, adapt the edit.

Objective:
Make the read_file "no such file" recovery hint more actionable by suggesting `rg --files` as a faster alternative to `list_files`, and adding a fallback instruction to search for the owning symbol instead of retrying the same path.

Why this matters:
The log_feedback lesson about retrying absent paths is currently only a behavioral prompt instruction — embedding it in the tool recovery hint makes it fire at the exact moment the agent encounters the failure, when the lesson is most actionable. This is the same principle as Day 114's lesson "A recovery instruction without timing is a tip, not a safety net."

Success Criteria:
- The read_file "no such file" recovery hint includes a suggestion to use `rg --files <pattern>` to verify path existence.
- The hint also includes a fallback: "search for the owning module or symbol instead of retrying the absent path."
- The existing test `test_targeted_recovery_hint_read_file_no_such_file` passes after updating the expected string.

Verification:
- cargo test test_targeted_recovery_hint_read_file -- --test-threads=1
- cargo test tool_wrappers -- --test-threads=1

Expected Evidence:
- The hint text in the test assertion matches the updated hint in `targeted_recovery_hint`.
- Future sessions show fewer "read or searched paths that did not exist" in log_feedback corrected lessons.

Implementation Notes:
- Edit the string literal at lines 1125-1129 in `src/tool_wrappers.rs`. The current hint is:
  ```
  "The file or directory path doesn't exist. Use `list_files` to \
   discover the correct path, or check for typos in the path \
   you provided. Verify parent directories exist."
  ```
- Replace with something like:
  ```
  "The file or directory path doesn't exist. Verify the path with \
   `rg --files | grep <pattern>` (faster than list_files for large repos), \
   or check for typos. If the file is missing entirely, search for the \
   owning module or symbol instead of retrying the absent path."
  ```
- Update `test_targeted_recovery_hint_read_file_no_such_file` (around line 3128) to match the new text.
