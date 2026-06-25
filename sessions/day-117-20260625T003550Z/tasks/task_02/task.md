Title: Wire bash tool failures into state events to close transcript-only gap
Files: src/tool_wrappers.rs, src/tools.rs
Issue: none
Origin: planner

Evidence:
- Trajectory: `transcript_only_failed_tool_count=2` — 2 tool failures found in transcripts but absent from state events
- Trajectory: `failed_tool_summary.bash_tool_error=2` — 2 bash commands failed during implementation
- Trajectory graph-derived pressure #5: "Reconcile transcript-only tool failures — Recent transcripts contained failed tool actions absent from state events"
- Assessment: "2 bash tool errors + 2 transcript-only failures — need to examine what commands failed"
- Capability fitness: task_success_rate=0.0 — failed tool calls that can't be diagnosed from state events block root-cause analysis

Edit Surface:
- src/tool_wrappers.rs — add ToolFailureTracker or wrapper that records ToolCallFailed events
- src/tools.rs — wire failure recording into StreamingBashTool or its wrapper chain

Verifier:
- cargo build && cargo test --lib tool_wrappers -- --test-threads=1
- Manual: yyds state tail --limit 50 | grep -i "ToolCallFailed" after a deliberate bash failure

Fallback:
- If the state recorder isn't accessible from the tool layer without a major refactor, record a state event from the prompt retry layer instead, or write a blocked note explaining the architectural gap.

Objective:
Ensure every bash tool failure produces a state event so post-mortem analysis can distinguish environment errors from logic errors without reading full transcripts.

Why this matters:
The trajectory shows 2 bash errors that were only visible in transcripts — state events had no record of them. This is a blind spot: if tool failures aren't in the state graph, the graph-derived task pressure can't detect recurring failure patterns, and the dashboard can't surface them. Fixing this improves task_verification_rate and coding_log_score by making failures diagnosable.

Success Criteria:
- A bash command that exits non-zero produces a ToolCallFailed or equivalent state event
- The event includes: command text (redacted), exit code, stderr summary
- Existing tests in tool_wrappers continue to pass

Verification:
- cargo build
- cargo test --lib tool_wrappers -- --test-threads=1
- Manual smoke test: run a bash command expected to fail, check `yyds state tail --limit 20` for failure event

Expected Evidence:
- After deployment, `yyds state graph hotspots` shows bash ToolCallFailed events in the event stream
- Future trajectory reports `transcript_only_failed_tool_count=0` for sessions after this lands
- State capture coverage remains at 1.0 (no regression)

Implementation Notes:
- The StreamingBashTool returns stdout/stderr/exit_code via a ToolOutput. The wrapper chain (GuardedTool, TruncatingTool, etc.) processes this output. Add failure recording at the wrapper level rather than inside the tool itself.
- Use `state::record(StateEvent::tool_call_failed(...))` or the existing StateRecorder pattern. Check how GuardedTool already records directory violations — follow the same pattern.
- Redact the command text to remove any API keys or secrets (use the existing redaction in `src/prompt_retry.rs` as a guide).
- Keep stderr summaries short (≤500 chars) to avoid bloating state events.
- The StateRecorder global is initialized in `src/state.rs`. Verify it's accessible from tool_wrappers before coding.
