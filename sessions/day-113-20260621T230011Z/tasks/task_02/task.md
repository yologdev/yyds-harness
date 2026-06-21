Title: Record bash non-zero exit codes as FailureObserved state events
Files: src/tools.rs, src/tool_wrappers.rs
Issue: none
Origin: planner

Evidence:
- Trajectory Day 113: "Bound failing shell commands before retrying (bash_tool_error=6): prefer bounded commands with explicit paths and inspect exit output before retrying broader checks"
- Trajectory Day 113: "Reconcile transcript-only tool failures (transcript_only_failed_tool_count=1): Recent transcripts contained failed tool actions absent from state evidence"
- `yyds state failures tools` returns empty (verified Day 113 23:04): 0 tool failures recorded in state
- `yyds state doctor` shows 0 failures recorded total (verified Day 113 23:04)
- But log feedback and transcripts show bash tool errors exist (6 in recent trajectory)
- The only `FailureObserved` emission path is the panic hook (src/state.rs:53). Non-panic bash failures (exit_code != 0) are captured in `ToolResult.details` (src/tools.rs:663) but never recorded as state `FailureObserved` events.
- `ToolFailureTracker` (src/tool_wrappers.rs:958) only counts failures for recovery-hint escalation; it does not emit state events.

Edit Surface:
- src/tools.rs, src/tool_wrappers.rs

Verifier:
- cargo test tools tool_wrappers -- --test-threads=1

Fallback:
- If `state::record` is not available from the tool layer (not initialized, wrong actor context), write session_plan/task_02_blocked.md explaining the architectural constraint. Do not restructure the state init flow for this task.

Objective:
Emit `FailureObserved` state events when `StreamingBashTool` executes a command that exits with a non-zero exit code, so that `yyds state failures tools` and trajectory/log feedback agree on tool failure counts.

Why this matters:
The trajectory reports bash tool errors but state shows zero failures. This mismatch means:
- `state failures tools` is unreliable for diagnosing bash-level problems
- Log feedback and trajectory can't cross-reference tool failures against state evidence
- The harness can't track which bash failure patterns recur across sessions
Closing this gap makes the state layer an honest record of tool execution outcomes.

Success Criteria:
- When a bash command exits non-zero, a `FailureObserved` state event is emitted with `Actor::Tool` (or appropriate actor) and a payload containing the tool name ("bash"), exit code, and command summary
- Exit code 0 commands do NOT emit FailureObserved (no false positives)
- Exit code 1 from commands like `grep` (no matches) is treated as a real failure for state purposes — the tool already formats it with "Exit code: 1" in the content, so it's visible to the agent as a failure
- `cargo test tools tool_wrappers` passes
- After the change, running a failing bash command through yyds should produce a visible FailureObserved event in `state tail`

Verification:
- cargo test tools tool_wrappers -- --test-threads=1
- cargo check

Expected Evidence:
- `yyds state failures tools` shows bash failures after a session where bash commands exited non-zero
- State events include `FailureObserved` records with tool=bash, exit_code=N payloads
- Trajectory bash_tool_error counts should correlate with state FailureObserved counts

Implementation Notes:
- The natural emission point is in `StreamingBashTool`'s `invoke` method (src/tools.rs, around line 653) where `exit_code != 0` is already checked. After formatting the output, add a `state::record(EventType::FailureObserved, Actor::Tool, payload)` call.
- Use `crate::state::record(...)` — check that `state::is_initialized()` returns true before recording (fail-soft: skip recording if state isn't initialized, don't panic). The state recorder is initialized early in agent setup, so it should be available during tool execution.
- The payload should be a JSON value with at minimum: `{"tool": "bash", "exit_code": N, "command_summary": "first 200 chars of command"}`
- Also consider extending `ToolFailureTracker.record_failure()` to optionally emit a state event. The tracker is called from `RecoveryHintTool` (src/tool_wrappers.rs:1129) when a tool fails. Adding state recording there would cover failures from all tool types, not just bash. However, for this task, keep it scoped to bash only — the tracker records tool names generically but doesn't have access to exit codes or command text.
- Do NOT change the RecoveryHintTool or general tool failure tracking — stay scoped to the bash exit-code path.
