Title: /compact shows summary of retained context after compaction
Files: src/commands_session.rs, src/prompt_utils.rs
Issue: none

## What

After `/compact` runs, show a brief summary of what topics and files are still present in the compacted conversation, so users can trust that important context wasn't lost.

## Why

When compaction reduces 50 messages to 15, users have no idea what survived. This creates anxiety about losing important context, which discourages using `/compact` when they should. Claude Code's compaction gives users confidence about what's retained. Showing "Still in context: auth refactor, src/main.rs, test plan" makes compaction trustworthy.

## How

1. **In `src/prompt_utils.rs`:**
   - Add `pub fn summarize_context_topics(messages: &[AgentMessage]) -> Vec<String>`:
     a. Scan all messages for file paths mentioned (look for patterns like `src/*.rs`, common path patterns in text content and tool calls)
     b. Scan tool_use calls for `read_file`, `write_file`, `edit_file` — extract file paths from params
     c. Collect unique file paths (deduplicated, max 10)
     d. Also extract topic keywords from user messages (first 5 user messages, take the first meaningful noun phrase or question)
     e. Return a vec of short topic strings like `["src/main.rs", "src/tools.rs", "auth refactor"]`

2. **In `src/commands_session.rs`:**
   - After successful compaction in `handle_compact`, call `summarize_context_topics` on the compacted messages
   - Display: 
     ```
     compacted (kept last 5): 42 → 12 messages, ~45K → ~12K tokens
     📋 Still in context: src/main.rs, src/tools.rs, auth discussion (3 files, 2 topics)
     ```
   - If no topics found, just show the existing line (don't add noise)

3. **Tests:**
   - Test `summarize_context_topics` with mock messages containing file paths in tool calls
   - Test with empty messages returns empty vec
   - Test deduplication and max cap

## Size estimate

~80 lines of new code. The topic extraction is the interesting part — keep it simple (file paths from tool calls + first words of user messages).
