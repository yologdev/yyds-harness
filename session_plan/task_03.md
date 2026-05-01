Title: Add /context files subcommand to show files in conversation
Files: src/commands_project.rs
Issue: none

Add a `/context files` subcommand that shows a summary of all files referenced in the current conversation — files the agent has read, written, or edited during the session, aggregated from tool call history.

This closes a UX gap vs Claude Code, which shows what files are "in context." Currently yoyo has `/changes` (which shows files the agent modified) and `/context` (which shows project context files like YOYO.md). Neither shows the full picture of what files the agent has interacted with.

### Changes to `src/commands_project.rs`:

1. **Add "files" to `CONTEXT_SUBCOMMANDS`** (~line 48):
   ```rust
   const CONTEXT_SUBCOMMANDS: &[&str] = &["system", "tokens", "files"];
   ```

2. **Add handler in `handle_context`** (~line 54):
   ```rust
   } else if args.starts_with("files") {
       show_context_files(agent);
   }
   ```

3. **Implement `show_context_files(agent: &Agent)`**:
   - Iterate over `agent.messages()` looking for tool use and tool results
   - Extract file paths from tool calls:
     - `read_file` → extract `path` from params
     - `write_file` → extract `path` from params
     - `edit_file` → extract `path` from params
     - `list_files` → extract `path` from params (the directory listed)
     - `search` → extract `path` from params if present
   - Deduplicate and sort file paths
   - Group by action type: read, written, edited, listed, searched
   - Display with color coding:
     ```
     Files in this conversation:
       📖 Read:     src/main.rs, src/tools.rs, Cargo.toml
       ✏️  Edited:   src/prompt.rs
       📝 Written:  src/new_file.rs
       📂 Listed:   src/, src/format/
       🔍 Searched: src/
     ```
   - If no files referenced, show: "(no files referenced yet)"

4. **Parse tool calls from messages**:
   The messages are `AgentMessage` variants. Look for `AgentMessage::Llm(Message::Assistant { content, .. })` where content blocks include `ContentBlock::ToolUse { name, input, .. }`. Extract file paths from `input["path"]` for file tools.

   Note: You'll need to check what `AgentMessage` and `Message` variants are available in yoagent 0.8.0. Use `grep -rn "enum AgentMessage\|enum Message\|ToolUse" ~/.cargo/registry/src/*/yoagent-0.8.0/src/` to find the right types.

5. **Add to help text** in `src/help.rs`:
   Find the `/context` help entry and add "files" to the subcommands list.

6. **Add tests:**
   - `test_context_files_subcommand_in_list` — verify "files" is in CONTEXT_SUBCOMMANDS
   - `test_show_context_files_no_panic` — basic smoke test with empty agent

### File limit note:
This task touches `src/commands_project.rs` primarily. If the help text update in `src/help.rs` would make it a 3rd file, skip the help update — it can be done in a future session.

### Acceptance criteria:
- `/context files` shows files the agent has interacted with in the current session
- Files are grouped by action type (read, edited, written, listed, searched)
- Deduplicated and sorted within each group
- Graceful handling when no files are referenced
- "files" appears in CONTEXT_SUBCOMMANDS
- `cargo build && cargo test && cargo clippy --all-targets -- -D warnings`
