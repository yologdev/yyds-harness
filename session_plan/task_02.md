Title: WebSearchTool — agent-callable web search tool
Files: src/tools.rs, src/agent_builder.rs
Issue: none

## What

Add a `WebSearchTool` that the agent can call during problem-solving to search the web. Register it in the tool list and BUILTIN_TOOL_NAMES.

## Why

This closes the biggest competitive gap identified in the assessment. The agent currently has no way to search for information during auto-fix loops, research tasks, or general problem-solving. Every competitor has this built in. After Task 1 lands the search engine functions, this task wires them into the agent's tool belt.

## Implementation

### 1. Create `WebSearchTool` struct in `src/tools.rs`

```rust
pub(crate) struct WebSearchTool;
```

Implement `yoagent::AgentTool` for it:

- **name:** `"web_search"`
- **description:** `"Search the web using DuckDuckGo. Returns a list of search results with titles, URLs, and snippets. Use this when you need to look up documentation, find solutions to errors, or research unfamiliar topics."`
- **parameters:** JSON schema with:
  - `query` (string, required): "The search query"
  - `max_results` (integer, optional): "Maximum number of results to return (default: 5, max: 20)"
- **execute:** 
  - Extract `query` from params (required — return error if missing)
  - Extract `max_results` from params (optional, default 5)
  - Call `commands_web::web_search_and_read(&query, max_results)` (from Task 1)
  - Return the formatted results string

### 2. Register in BUILTIN_TOOL_NAMES (`src/agent_builder.rs`)

Add `"web_search"` to the `BUILTIN_TOOL_NAMES` array. This prevents MCP tool name collisions.

### 3. Add to `build_tools` function (`src/tools.rs`)

Add `WebSearchTool` to the tools vector in `build_tools`, wrapped with:
- `with_truncation` (respects max_tool_output)
- `with_recovery_hints` (failure tracker)
- `maybe_hook` (hook system)

Place it after the SearchTool (file search) and before ask_user.

### 4. Add to sub-agent tools (`build_sub_agent_tool`)

Add `Arc::new(WebSearchTool)` to the `child_tools` vec in `build_sub_agent_tool` so sub-agents can also search the web.

### 5. Tests

- Test that `WebSearchTool` appears in `build_tools` output (check tool names)
- Test that `"web_search"` is in `BUILTIN_TOOL_NAMES`
- Test tool parameter validation (missing query → error)
- Test the tool's JSON schema has correct required/optional fields

### Notes
- This task depends on Task 1 landing the `web_search_and_read` function in `commands_web.rs`
- The tool must be added to BUILTIN_TOOL_NAMES **and** build_tools — missing either causes subtle bugs
- No new dependencies needed
