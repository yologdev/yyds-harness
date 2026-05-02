Title: Extract event handlers from handle_prompt_events into focused functions
Files: src/prompt.rs
Issue: none

The 466-line `handle_prompt_events` function (line 651) is the largest single function in the codebase. It contains a `loop { tokio::select! { match event { ... } } }` with 12+ match arms, each handling a different `AgentEvent` variant. The local state (15+ variables) and the match arms are tightly coupled, making the function hard to read and modify.

**What to do:**

1. Create a `PromptEventState` struct that bundles the 15+ local variables currently declared at the top of `handle_prompt_events`:
   - `usage`, `collected_text`, `in_text`, `last_was_newline`, `had_text`
   - `spinner`, `think_filter`
   - `audit_inflight`, `tool_progress_timers`, `deferred_bash_timers`
   - `batch_count`, `batch_tool_names`, `batch_results`, `batch_start`
   - `turn_number`, `retriable_error`, `overflow_error`
   - `last_tool_error`, `last_tool_name`

2. Extract the largest match arms into methods on `PromptEventState`:
   - `handle_tool_execution_start(...)` — the `AgentEvent::ToolExecutionStart` arm (~80 lines)
   - `handle_tool_execution_end(...)` — the `AgentEvent::ToolExecutionEnd` arm (~60 lines)
   - `handle_tool_execution_update(...)` — the `AgentEvent::ToolExecutionUpdate` arm (~30 lines)
   - `handle_message_update_text(...)` — the text delta `MessageUpdate` arm (~65 lines)
   - `handle_agent_end(...)` — the `AgentEvent::AgentEnd` arm (~60 lines)

3. The remaining small arms (`TurnStart`, `TurnEnd`, `MessageStart`, `MessageEnd`, `InputRejected`, `ProgressMessage`, error `MessageUpdate`) can stay inline since they're 1-5 lines each.

4. The main `handle_prompt_events` function should shrink to roughly:
   - State initialization (create `PromptEventState`)
   - The event loop with small match arms that delegate to methods
   - The ctrl+c handler
   - The final return logic

5. Keep `PromptEventState` private (not `pub`) — it's an implementation detail of `prompt.rs`.

6. Add no new tests — this is a pure refactor with zero behavior change. Existing tests must continue to pass.

**Sizing:** This touches only `src/prompt.rs`. The struct + 5 method extractions should be well within 20 minutes. Each extraction is mechanical: cut the match arm body, paste into a method, replace with a method call.
