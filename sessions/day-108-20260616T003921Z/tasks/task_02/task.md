Title: Capture bash exit_code in CommandCompleted state events to close transcript-state gap
Files: src/prompt.rs
Issue: none
Origin: planner

Objective:
Add `exit_code` to the `CommandCompleted` state event payload so that bash command failures (non-zero exit codes) are visible in state evidence, not just in transcripts. This closes the gap where the dashboard detects failed bash commands in transcripts that appear as successful tool calls in state events.

Why this matters:
The trajectory reports `transcript_only_failed_tool_count=4` as graph pressure: "Recent transcripts contained failed tool actions absent from state evidence — state/transcript gap." The log feedback lesson says: "prefer bounded commands with explicit paths and inspect exit output before retrying broader checks" — but currently state events don't capture exit codes at all, making it impossible for dashboards to distinguish "tool call threw an error" from "tool call succeeded but the bash command inside it failed."

Currently, when a bash command exits with code 1, the StreamingBashTool returns the output normally (not as an error), so `is_error` is `false` in both ToolCallCompleted and CommandCompleted. The exit_code lives in `result.details` but is discarded before the state event is recorded. The dashboard has to parse transcripts to find the "Exit code: 1" line — creating the transcript-state gap.

Adding `exit_code` (or `details`) to the CommandCompleted payload makes this information available in state events, enabling dashboards and feedback scripts to detect bash command failures directly from state evidence.

Success Criteria:
- `CommandCompleted` state events for bash tool calls include `exit_code` when available from the tool result details.
- Existing state event consumers (dashboards, `state why`, eval scripts) are not broken by the new field.
- `cargo build && cargo test` pass.

Verification:
- cargo build
- cargo test --lib
- cargo check

Expected Evidence:
- Future dashboard runs show `transcript_only_failed_tool_count` decreasing as bash failures become visible in state events.
- The `failed_tool_summary.bash_tool_error` metric in state graphs can be computed from state events rather than requiring transcript parsing.

Implementation Notes:
The change location is `src/prompt.rs`, around lines 869-877, inside the `AgentEvent::ToolExecutionEnd` handler where CommandCompleted is emitted.

Current code:
```rust
if tool_name == "bash" {
    crate::state::record(
        crate::state::EventType::CommandCompleted,
        crate::state::Actor::Tool,
        serde_json::json!({
            "tool_call_id": tool_call_id.clone(),
            "is_error": is_error,
            "result_preview": preview.clone(),
        }),
    );
```

The `result` value (type `ToolResult`) has a `.details` field: `Option<serde_json::Value>`. When tool_name is "bash", `result.details` contains `{"exit_code": N, "success": bool}`.

Extract exit_code from `result.details` and add it to the payload:

```rust
if tool_name == "bash" {
    let exit_code = result.details.as_ref()
        .and_then(|d| d.get("exit_code"))
        .and_then(|v| v.as_i64());
    let mut cmd_payload = serde_json::json!({
        "tool_call_id": tool_call_id.clone(),
        "is_error": is_error,
        "result_preview": preview.clone(),
    });
    if let Some(ec) = exit_code {
        cmd_payload["exit_code"] = serde_json::json!(ec);
        cmd_payload["success"] = serde_json::json!(ec == 0);
    }
    crate::state::record(
        crate::state::EventType::CommandCompleted,
        crate::state::Actor::Tool,
        cmd_payload,
    );
```

Only `exit_code` is strictly needed; `success` is a convenience for dashboard consumers. If `result.details` doesn't contain `exit_code` (non-bash tools, or tools that don't set details), the field is simply omitted — backward compatible.

Keep the change minimal — do not refactor or restructure the surrounding code. This is a pure data-capture improvement.
