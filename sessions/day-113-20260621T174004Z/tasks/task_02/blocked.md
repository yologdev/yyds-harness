# Task blocked by analysis-only implementation attempt

The implementation agent exited without landing file progress or emitting
TASK_TERMINAL_EVIDENCE on the first attempt. The harness stopped the task
instead of spending a second attempt on more analysis.

This task should be replanned with narrower scope, clearer owning files, or
stronger pre-confirmed evidence before another implementation attempt.

Recent transcript tail:
```
But wait, I also need to add the `state` import to tool_wrappers.rs. Let me also check what header imports are there...

Now let me implement the changes. My plan:

1. **src/tool_wrappers.rs**: 
   - Add a `state::record` call in RecoveryHintTool.execute() for the `Failed(msg)` error path
   - Add a `state::record` call for the `other` error path
   - Use `crate::state::record(crate::state::EventType::FailureObserved, crate::state::Actor::Harness, payload)` (fully qualified to avoid adding imports)

2. **src/prompt.rs**:
   - Remove the FailureObserved recording at lines 899-910 (tool failures)
   - Keep the InputRejected FailureObserved at lines 958-966
   - Add FailureObserved recording in handle_stream_json_events for tool failures (since this path sets last_tool_error but doesn't record FailureObserved... wait, RecoveryHintTool already records it!)
   
   Actually, since RecoveryHintTool records FailureObserved when a tool fails, and handle_stream_json_events processes the same tools, handle_stream_json_events doesn't need to add FailureObserved. The RecoveryHintTool recording covers it.

   But wait - handle_stream_json_events currently sets `last_tool_error` at line 1714, creating the transcript-only failures. Even with RecoveryHintTool recording FailureObserved, the transcript error status is still tracked. So the gap is filled.

3. **Test**: Add a test in tool_wrappers.rs that verifies RecoveryHintTool records a FailureObserved state event. But wait, `state::record` is a no-op when no recorder is initialized (which is the case in tests). So I can't directly test state event recording in unit tests.

   Let me think about this... I could:
   - Make the test verify that the attempt count is recorded and the error message is enriched (which is already tested)
   - Add a unit test for the FailureObserved recording by mocking or initializing the state recorder
   - Or skip the state event test and just verify through the existing tests

   Actually, let me look at how other tests verify state events.

Let me now also think about whether removing FailureObserved from handle_prompt_events could break existing tests. Let me search for tests in prompt.rs that verify FailureObserved recording.
────────────────────────────────────────────────────────────────────────────────


  ╭─ Turn 21 ────────────────────────────────────────────────────────────╮
  ▶ search 'FailureObserved' in src/prompt.rs ✓ (51ms)

── Thinking ────────────────────────────────────────────────────────────────
There are only two references to FailureObserved in prompt.rs (at lines 901 and 960), and they're both in the `handle_prompt_events` function. No tests in prompt.rs directly reference FailureObserved. So removing the tool-failure recording (line 901) shouldn't break existing tests.

Now let me proceed with the implementation. Let me also set up a todo list:

1. Add FailureObserved recording in RecoveryHintTool.execute() for both error paths
2. Remove FailureObserved recording from handle_prompt_events for tool failures (lines 899-910)
3. Add FailureObserved recording in handle_stream_json_events for tool failures
4. Add a unit test for RecoveryHintTool's FailureObserved recording (or a verification test)
5. Verify with cargo build && cargo test

Wait, for point 3: handle_stream_json_events also needs FailureObserved because the RecoveryHintTool records it at the tool execution level, not at the event handler level. So both handle_prompt_events and handle_stream_json_events would receive FailureObserved from RecoveryHintTool. But handle_prompt_events would double-record if I don't remove it.

Actually, let me reconsider. The task says to add FailureObserved in RecoveryHintTool AND keep prompt.rs's recording but make sure they don't double-count.

Hmm, one approach: instead of removing FailureObserved from handle_prompt_events, I could detect whether the tool failure was already recorded by RecoveryHintTool. One way: RecoveryHintTool sets a marker in the ToolResult or ToolError. But ToolError doesn't support custom metadata.

Another approach: RecoveryHintTool records FailureObserved with a specific payload field (like `source: "recovery_hint_tool"`), and handle_prompt_events records with `source: "tool"`. Downstream tools can deduplicate. But this doesn't satisfy "No double-counting."

Actually, the simplest and most correct approach is:
1. Add FailureObserved in RecoveryHintTool (with payload including tool_name, attempt, error_preview)
2. Remove FailureObserved from handle_prompt_events (for tool failures only, keep InputRejected)
3. RecoveryHintTool covers all major tools, and the unwrapped ones (ask_user, todo) are simple enough that their failures are rare and less diagnostic

Let me go with this approach. Let me also add FailureObserved to handle_stream_json_events as a safety net, in case something bypasses RecoveryHintTool.

Wait, nothing can bypass RecoveryHintTool for wrapped tools. The tool is the RecoveryHintTool wrapper, and execute() is the entry point. So if I add FailureObserved in RecoveryHintTool, all wrapped tools are covered.

Let me just implement it.

Actually, I realize I need to also handle one edge case: what if a tool fails with an error that RecoveryHintTool doesn't catch? RecoveryHintTool only catches `ToolError::Failed(msg)` and non-`Failed` errors (`NotFound`, `InvalidArgs`, `Cancelled`). If there's some other error type I'm not aware of, it would propagate up without being recorded. But since RecoveryHintTool's match is exhaustive (Ok, Err(Failed), Err(other)), this should be fine.

Let me now implement the changes.
────────────────────────────────────────────────────────────────────────────────

Now I have a complete understanding of the failure tracking paths. Let me set up my task list and implement:
  ╭─ Turn 22 ────────────────────────────────────────────────────────────╮
  ▶ todo ✓ (52ms)
  ▶ todo ✓ (56ms)
  ▶ todo ✓ (96ms)
  ▶ todo ✓ (142ms)
  ▶ todo ✓ (98ms)
  ▶ todo ✓ (142ms)
  6 tools completed in 1.0s (6 ✓)


```
