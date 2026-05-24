Title: Per-tool usage summary in /cost and /tokens
Files: src/format/cost.rs, src/commands_info.rs
Issue: none

## What

Add a per-tool call summary to `/cost` output, showing which tools were called and how many times during the session. This directly addresses the competitive gap identified in the assessment: "No per-tool/per-category cost breakdown — users can't see what's expensive."

Claude Code now breaks down costs by skills, subagents, plugins, and MCP servers. We can start by showing tool-call frequency, which tells users what patterns their session followed (e.g., heavy on `bash` vs heavy on `edit_file`).

## Implementation

### In `src/format/cost.rs`:

1. Add a new struct `ToolCallSummary` with fields:
   - `name: String` — the tool name
   - `calls: usize` — number of calls
   - `errors: usize` — number of error results

2. Add function `extract_tool_call_summary(messages: &[AgentMessage]) -> Vec<ToolCallSummary>`:
   - Iterate over all messages, find `Message::ToolResult` entries
   - Count calls per `tool_name`, track `is_error` counts
   - Return sorted by call count descending
   - Skip if empty

3. Add function `format_tool_call_summary(summary: &[ToolCallSummary]) -> String`:
   - Format as a compact table:
     ```
     Tool usage:
       bash           12 calls
       edit_file       8 calls (1 error)
       read_file       5 calls
       search          3 calls
     ```
   - If any tool has errors, show the error count in parentheses
   - Use DIM formatting consistent with existing cost output

### In `src/commands_info.rs`:

4. In `handle_cost()`, after the per-turn breakdown section, add the tool call summary:
   ```rust
   let tool_summary = extract_tool_call_summary(messages);
   if !tool_summary.is_empty() {
       println!();
       println!("{}", format_tool_call_summary(&tool_summary));
   }
   ```

5. Also add the tool summary to `handle_tokens()` (which takes `agent` and can get messages from it). If `handle_tokens` doesn't have access to messages, pass them in or extract from the agent.

### Tests

Add tests in `src/format/cost.rs`:
- `test_extract_tool_call_summary_empty` — no messages → empty vec
- `test_extract_tool_call_summary_counts` — mix of tool results → correct counts
- `test_extract_tool_call_summary_errors` — some is_error=true → correct error counts  
- `test_format_tool_call_summary` — verify formatting output
- `test_extract_tool_call_summary_sorted` — results sorted by call count desc

No docs changes needed — this enhances existing commands, not adding new ones.
