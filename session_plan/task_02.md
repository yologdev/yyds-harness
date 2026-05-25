Title: /compact --preview to show what compaction would do
Files: src/commands_session.rs, src/dispatch.rs
Issue: none

## What

Add a `--preview` flag to the `/compact` command that shows what a compaction would look like without actually performing it: estimated token savings, number of messages that would be compressed, and a brief topic summary of what's in the conversation.

## Why

Compaction is currently a black box — users run `/compact` and hope for the best, or avoid it because they don't know what they'll lose. Commercial agents handle this more gracefully by showing context state. A preview gives users confidence to compact at the right time and helps them understand what's in their context window.

## Implementation

1. **In `src/commands_session.rs`**, modify `parse_compact_arg` to also recognize `--preview` as a variant:
   - Add `CompactArg::Preview` variant to the enum
   - Parse `"--preview"` or `"preview"` in `parse_compact_arg`

2. **Add a new function** `fn handle_compact_preview(agent: &Agent)`:
   - Get current messages via `agent.messages()`
   - Count total messages and compute total tokens via `total_tokens()`
   - Estimate post-compaction size: use the same threshold logic as `compact_agent` to determine what would be kept vs compressed. A rough estimate is fine (e.g., "keeping last N messages at ~X tokens, compressing M older messages")
   - Generate a brief topic summary by scanning message content for key topics (file names mentioned, tool calls made, questions asked) — just the last 5-10 unique file paths and a count of tool calls
   - Print the preview in a readable format:
     ```
     📋 Compact preview:
       Current: 47 messages, ~28K tokens (72% of context)
       After:   ~15 messages, ~12K tokens (estimated)
       Savings: ~16K tokens freed
       
       Would compress: 32 older messages covering:
         • File edits: src/watch.rs, src/tools.rs, src/main.rs
         • Tool calls: 18 bash, 12 edit_file, 5 read_file
         • Topics: watch mode fixes, test additions
       Would keep: last 15 messages (recent conversation)
     ```

3. **In `handle_compact`**, add a branch at the top for `CompactArg::Preview` that calls `handle_compact_preview` and returns early.

4. **In `src/dispatch.rs`**, no changes needed — the `/compact` command already routes to `handle_compact`.

5. **Add tests**:
   - Test that `parse_compact_arg("--preview")` returns `CompactArg::Preview`
   - Test that `parse_compact_arg("preview")` returns `CompactArg::Preview`
   - Test that the preview function doesn't panic on empty message lists

## Constraints
- Only touch `src/commands_session.rs` and (if needed for routing) `src/dispatch.rs`
- The preview must NOT actually perform compaction — read-only operation
- Keep estimates rough but honest — better to say "approximately" than to be precise but wrong
- Topic detection should be simple: scan for file paths in tool results, count tool call types
