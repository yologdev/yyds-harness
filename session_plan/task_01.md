Title: Enhanced edit_file error context with line numbers and nearest match
Files: src/tool_wrappers.rs
Issue: none

When `edit_file` fails because `old_text` was not found, the current error from yoagent shows a basic "Did you mean" suggestion using `find_similar_text`. The RecoveryHintTool adds generic advice ("Use read_file to see current contents"). Neither tells the agent *where* in the file the closest match is.

**What to build:**

Add a new tool wrapper `SmartEditTool` (or extend the existing `RecoveryHintTool` logic for `edit_file` specifically) that intercepts `edit_file` `ToolError::Failed` responses containing "old_text not found" and augments them with:

1. **Line numbers** where the closest match was found (using first-line matching with whitespace normalization)
2. **Actual content snippet** (3-5 lines) at that location so the agent can see what's really there
3. **Whitespace diff indicator** if the mismatch is purely whitespace (e.g., "The text matches at line 42 but indentation differs")

**Implementation approach:**

In `tool_wrappers.rs`, create a new wrapper `SmartEditTool` that:
- Wraps the inner edit_file tool
- On `ToolError::Failed` where the message contains "not found":
  - Reads the file from the `path` parameter
  - Extracts the first non-empty line of `old_text`
  - Searches the file for lines containing that text (case-sensitive)
  - If found, reports: "Nearest match at line N:\n```\n<actual lines>\n```"
  - If the match differs only in whitespace, adds: "Hint: the text exists but indentation differs"
- On success or other errors, passes through unchanged

Wire it into `build_tools()` — it should wrap `edit_file` before the `RecoveryHintTool` wrapper (so both layers fire: SmartEditTool provides context, RecoveryHintTool provides escalating strategy).

**Constraints:**
- Max 5 lines of context shown in the error augmentation
- Don't read files larger than 100KB (skip augmentation for huge files)
- The wrapper must not panic on any edge case (missing file, binary file, etc.)

**Tests:**
- Test that when old_text's first line exists in the file at a known position, the error includes "line N"
- Test that whitespace-only mismatch is detected
- Test that the wrapper passes through successful results unchanged
- Test that non-"not found" errors pass through unchanged

This directly reduces the number of failed edit_file retries the agent needs, which is the #1 friction point in coding agent workflows.
