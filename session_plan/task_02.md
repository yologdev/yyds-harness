Title: Enhanced tool recovery with concrete alternative tool suggestions
Files: src/prompt_retry.rs
Issue: none (Priority Queue gap #2: graceful degradation on partial tool failures)

## Problem

The competitive gap scorecard (CLAUDE_CODE_GAP.md Priority Queue #2) notes:
"Full graceful degradation on partial tool failures — provider fallback covers
hard API errors, but there's no story for 'this tool call failed, try a
different tool that achieves the same effect.'"

Currently `tool_recovery_hint()` returns generic advice like "The edit failed.
Use read_file to see current contents, then retry with the exact text." But the
auto-retry prompt doesn't concretely suggest switching to an alternative tool.
The model gets vague advice and often just retries the same failing approach.

## Solution

Enhance `build_auto_retry_prompt()` in `src/prompt_retry.rs` to include
concrete alternative tool suggestions when a specific tool fails. The key
insight: each tool has a natural fallback that achieves a similar effect.

### Implementation Details

1. **Enhance `tool_recovery_hint()` to return structured suggestions** with both
   the diagnostic message AND a concrete alternative approach:

   ```
   edit_file failed → "Try write_file instead: read the file with read_file to get
   the current contents, make the edit in the full text, then use write_file to
   replace the entire file."

   read_file failed → "Try bash instead: use `cat <path>` or `head -n 100 <path>`
   to read the file contents directly."

   search failed → "Try bash instead: use `grep -rn '<pattern>' <path>` for regex
   search, or `find . -name '<pattern>'` for file name search."

   write_file failed → "Try bash instead: use `cat > <path> << 'HEREDOC'\n...\nHEREDOC`
   to write file contents."

   rename_symbol failed → "Try search + edit_file instead: use search to find all
   occurrences of the symbol, then use edit_file on each file to replace them."

   bash failed → "Try a simpler command: break the command into smaller steps,
   check if the binary exists with `which <cmd>`, or try an alternative tool
   (e.g., read_file instead of cat, search instead of grep)."
   ```

2. **Add attempt-aware escalation**: On the first retry (attempt 1), give the
   standard hint. On attempt 2+, escalate to the alternative tool suggestion.
   This prevents premature tool-switching on transient failures.

   Modify `build_auto_retry_prompt` to accept the attempt number and select
   the appropriate hint level:
   - Attempt 1: Current diagnostic hint ("The edit failed, check the text matches")
   - Attempt 2+: Full alternative tool suggestion ("Try write_file instead...")

3. **Refactor `tool_recovery_hint` signature** to:
   ```rust
   pub fn tool_recovery_hint(tool_name: &str, attempt: usize) -> &'static str
   ```
   - attempt == 1: return diagnostic hint (current behavior)
   - attempt >= 2: return alternative tool suggestion

4. **Update `build_auto_retry_prompt`** to pass the attempt number to
   `tool_recovery_hint`. The attempt is already available as a parameter.

5. **Update existing tests** for `tool_recovery_hint` to cover both attempt levels.
   Add new tests for:
   - `tool_recovery_hint("edit_file", 1)` returns diagnostic
   - `tool_recovery_hint("edit_file", 2)` returns alternative tool suggestion
   - `tool_recovery_hint("bash", 2)` returns simpler-command advice
   - All known tools covered at both levels

### Scope

This is a focused change to ONE file (`src/prompt_retry.rs`). No changes to the
retry loop itself — just better content in the retry prompts.
