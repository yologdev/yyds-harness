Title: Inline tool recovery hints in tool error responses
Files: src/tool_wrappers.rs, src/prompt_retry.rs
Issue: none

## Context

The graceful degradation gap (🟡 in CLAUDE_CODE_GAP.md) says "not yet full fallback on partial tool failures." Day 70 added `tool_recovery_hint()` in `prompt_retry.rs` which provides escalating advice (attempt 1: fix the error; attempt 2+: try a different tool). However, these hints are ONLY used when building retry prompts — they're not included in the tool error result that the LLM sees inline during normal conversation flow.

This means the agent only gets tool recovery hints during auto-retry scenarios, not during normal interactive conversation where a tool fails. Claude Code naturally handles this — when a tool fails, the model gets guidance on alternatives directly in the error response.

## What to do

### 1. Add a ToolFailureTracker to tool_wrappers.rs

Create a simple `ToolFailureTracker` (using `Arc<Mutex<HashMap<String, u32>>>`) that tracks consecutive failures per tool name. When a tool succeeds, reset its counter. When it fails, increment.

### 2. Create a new wrapper: `RecoveryHintTool`

A new tool wrapper in `tool_wrappers.rs` that:
- Wraps any `AgentTool`
- On success: resets the failure counter for this tool name, returns the result unchanged
- On failure: increments the failure counter, appends the recovery hint from `prompt_retry::tool_recovery_hint(name, attempt)` to the `ToolError::Failed` message

This is a thin wrapper — it doesn't change tool behavior, just enriches error messages.

### 3. Wire it into tool building

Add a `with_recovery_hints` helper function (similar to `with_truncation`, `with_auto_check`) that wraps a tool with `RecoveryHintTool`. The tracker is shared across all tools so cross-tool failure counts work correctly.

Don't wire it into `build_tools` yet — just make the wrapper and helper available. Wiring it in would touch `tools.rs` (third file) and the concern is orthogonal. The wrapper should be ready to use.

## Tests to add

- `test_recovery_hint_tool_success_resets_counter` — verify counter resets on success
- `test_recovery_hint_tool_appends_hint_on_failure` — verify the error message includes the recovery hint
- `test_recovery_hint_tool_escalates_on_repeated_failure` — verify attempt 2 gets a different (more aggressive) hint than attempt 1
- `test_tool_failure_tracker_independent_per_tool` — verify different tool names track independently

## What NOT to do

- Don't modify `tools.rs` or `build_tools` — that's a separate task
- Don't change `tool_recovery_hint` itself — it already works well
- Don't add any new dependencies
