Title: Extend search tool with binary-match recovery hints
Files: src/tools.rs
Issue: none
Origin: planner (refined from harness-seed — original seed targeted api_key_present:false crashes the fresh assessment classifies as CI/automation noise)

Objective:
Add binary-match detection and recovery hints to the search tool's error/output path, extending Day 105's regex-error recovery pattern to the second-most-common tool failure category (search_binary_match=19).

Why this matters:
The state/trajectory snapshot shows search_regex_error=57 as the top tool failure and search_binary_match=19 as the second. Day 105 already shipped regex-error recovery hints in the search tool error path (lines 244-257 of src/tools.rs). Binary matches happen because `build_project_rg_args` does not pass `-I` (ignore binary files), so ripgrep reports "binary file matches" lines that the agent receives as low-value results. Adding a recovery hint (or the `-I` flag) prevents these 19 failures from recurring.

Success Criteria:
- When rg reports binary file matches, the search result includes a hint about using `--binary` or `-I` to control binary file handling, OR binary files are skipped entirely (matching grep's `-I` behavior).
- Existing search behavior for text files is unchanged.
- The change is testable: search for a pattern that only exists in binary files produces a useful hint rather than bare "binary file matches" output.

Verification:
- cargo build
- cargo test tools
- cargo test --lib -- search

Expected Evidence:
- Task lineage links src/tools.rs to this task.
- Future state runs show reduced search_binary_match failures (from 19 toward 0).
- The search tool failure dashboard metric shifts down after the change lands.

Implementation Notes:
The fix has two possible approaches (choose the simpler one):
A) Add `-I` flag to `build_project_rg_args` (line 309) — this matches grep's behavior and prevents binary matches entirely. Simplest, fewest lines, prevents the failure class.
B) Detect binary match lines in stdout (look for "binary file matches" or "Binary file") and add a recovery hint similar to the regex-error hint pattern at lines 244-257. More defensive, but two detection paths to maintain.

Prefer approach A: consistent with grep, one line change, deterministic behavior. If that breaks any existing test, fall back to approach B.

The `build_project_grep_args` already passes `-I` (line 352). Adding `-I` to `build_project_rg_args` brings rg in line with grep.
