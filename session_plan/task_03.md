Title: Add /history detail subcommand for per-turn conversation breakdown
Files: src/commands_session.rs, src/dispatch.rs, src/help.rs
Issue: none

## Goal

`/history` currently shows a minimal one-line-per-message list (`[role] preview`). Users need to understand what's consuming their context window. Add `/history detail` that shows a per-turn breakdown with tools used, token counts, and turn summaries.

## Implementation

### 1. Add `handle_history_detail` in `src/commands_session.rs`

Create a new public function next to the existing `handle_history`:

```rust
pub fn handle_history_detail(agent: &Agent) {
    let messages = agent.messages();
    if messages.is_empty() {
        println!("{DIM}  (no messages in conversation){RESET}\n");
        return;
    }
    
    // Group messages into "turns": a turn = user message + assistant response
    // Walk through messages, pairing them.
    
    // For each pair, display:
    //   Turn 1
    //     You:   "explain the login flow" (42 tok in)
    //     Agent: 3 tool calls (bash ×2, read_file ×1), 1,247 tok out
    //
    //   Turn 2  
    //     You:   "now fix the auth bug" (28 tok in)
    //     Agent: 5 tool calls (edit_file ×3, bash ×2), 2,891 tok out
    //
    //   Total: 2 turns, ~4,208 tokens used

    // Extract tool use counts from assistant messages:
    // Iterate Content blocks, count ToolUse by name
    // Use HashMap<String, usize> to group tool calls
    
    // Format tool summary: "bash ×2, read_file ×1" 
    // or "no tool calls" if none
    
    // Truncate user text preview to ~60 chars with ellipsis
    
    // At the end, show total turns and total tokens
}
```

### 2. Modify routing in `src/dispatch.rs`

Change the existing `/history` match arm (around line 317):

From:
```rust
"/history" => {
    commands::handle_history(ctx.agent);
    CommandResult::Continue
}
```

To:
```rust
s if s == "/history" || s.starts_with("/history ") => {
    let sub = s.strip_prefix("/history").unwrap_or("").trim();
    if sub == "detail" {
        commands::handle_history_detail(ctx.agent);
    } else {
        commands::handle_history(ctx.agent);
    }
    CommandResult::Continue
}
```

### 3. Update help in `src/help.rs`

- Add `/history detail` to the session management section of /help
- Add description: "Show per-turn breakdown with tools used and token counts"
- If there are completions for /history, add "detail" as a subcommand

### 4. Extracting tool counts from assistant messages

```rust
use std::collections::HashMap;
use yoagent::types::{AgentMessage, Content, Message};

fn count_tools(msg: &AgentMessage) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    if let AgentMessage::Llm(Message::Assistant { content, .. }) = msg {
        for block in content {
            if let Content::ToolUse { name, .. } = block {
                *counts.entry(name.clone()).or_insert(0) += 1;
            }
        }
    }
    counts
}
```

### 5. Tests in `src/commands_session.rs`

- `test_handle_history_detail_empty` — call with agent that has no messages, verify no panic
- `test_count_tools_from_assistant` — build a mock assistant message with tool_use blocks, verify correct counts
- `test_format_tool_summary` — verify "bash ×2, read_file ×1" formatting

## Verification

```bash
cargo build && cargo test && cargo clippy --all-targets -- -D warnings
```

Exactly 3 source files modified: `commands_session.rs`, `dispatch.rs`, `help.rs`.
