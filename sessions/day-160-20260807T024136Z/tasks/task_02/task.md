Title: Add tool-call lineage to panic-hook FailureObserved payload
Files: src/state.rs, src/tool_wrappers.rs
Issue: none
Origin: planner

Evidence:
- Day 160 assessment §Graph Hotspots: "18 agent_error_exit events with zero inbound tool-call relations — these are failure events produced without tool-call lineage, suggesting crash/panic paths that recorded failure but couldn't trace the tool that caused it."
- `yyds state why last-failure` shows `source=unknown, class=unknown` for panic-induced failures — the FailureObserved payload has `failure_class: "rust_panic"` with `panic_message` and `panic_location` but no indication of which tool was executing when the panic occurred.
- State graph analysis confirms: 18 events with EventType=FailureObserved and payload.failure_class="rust_panic" but zero inbound edges from tool-call events — making root-cause analysis of panic-induced failures harder than necessary.
- The existing `CURRENT_MODEL_CALL_ID` thread-local (state.rs line 33) proves the pattern works: a thread-local set before an operation and cleared after. The panic hook already reads it (state.rs line 68) to close model call lifecycle. The same pattern can track active tool name.

Edit Surface:
- src/state.rs
- src/tool_wrappers.rs

Verifier:
- cargo test state tool_wrappers

Fallback:
- If `yyds state doctor` shows zero `agent_error_exit` events from the last 10 sessions (i.e., no recent panics), mark this task obsolete — the diagnostic improvement has no recent data to act on.
- If the `RecoveryHintTool::invoke` path turns out to be async and requires Send bounds that thread-locals can't satisfy, fall back to recording the tool name in the FailureObserved payload from the tool wrapper itself (add a try-catch around `self.inner.invoke()` that records a FailureObserved with tool_name on panic).

Objective:
When a Rust panic occurs during tool execution, the FailureObserved event payload includes the name of the tool that was executing, enabling state-graph analysis to link panic failures to specific tools.

Why this matters:
Panic-induced failures are the hardest to diagnose because they leave minimal evidence — a stack trace and a panic message, but no tool-call context. Without tool lineage, the state graph shows isolated FailureObserved nodes with no inbound edges from tool-call events, making it impossible to answer "which tool keeps crashing?" from state evidence alone. Adding tool_name to the panic FailureObserved payload creates the missing edge, enabling future sessions to detect patterns like "bash tool panics on specific input shapes" or "edit_file panics on empty files."

Success Criteria:
- When a panic occurs inside a tool wrapped by RecoveryHintTool, the FailureObserved payload includes a `tool_name` field with the tool's name.
- The `CURRENT_TOOL_NAME` thread-local is set before `self.inner.invoke()` and cleared in a finally/guard pattern so it never leaks between tool calls.
- The panic hook includes `tool_name` in the FailureObserved payload only when set (absent when panic occurs outside tool execution).
- Existing tests pass. New test verifies that panic-hook payload includes tool_name when CURRENT_TOOL_NAME is set.

Verification:
- cargo test state tool_wrappers
- cargo clippy -- -D warnings  (confirm no new warnings from thread-local usage)

Expected Evidence:
- After the fix, a simulated panic during tool execution produces a FailureObserved event with `tool_name: "bash"` (or whichever tool was active) in the payload.
- Future `yyds state graph` shows inbound edges from tool-call events to panic-induced FailureObserved events, closing the lineage gap.
- The `agent_error_exit` count in state diagnostics stops growing for new sessions (existing historical events remain unlinked — only new panics get the field).

Implementation Notes:
- Add to `src/state.rs`:
  - `thread_local! { static CURRENT_TOOL_NAME: Cell<Option<String>> = const { Cell::new(None) }; }` (modeled on CURRENT_MODEL_CALL_ID at line 33)
  - `pub fn set_current_tool_name(name: String)` — sets the cell
  - `pub fn clear_current_tool_name()` — clears the cell
- In the panic hook (state.rs line 58-63): add `tool_name` to the FailureObserved payload by reading CURRENT_TOOL_NAME. Use `"unknown"` or omit the field when the cell is None (panic outside tool execution).
- In `src/tool_wrappers.rs`: in `RecoveryHintTool::invoke()` (line 1172 area), wrap `self.inner.invoke()` with:
  1. `crate::state::set_current_tool_name(self.inner.name().to_string())`
  2. Call `self.inner.invoke(params).await`
  3. In a finally/guard: `crate::state::clear_current_tool_name()`
  - Use a scope guard pattern: a struct that calls clear_current_tool_name on drop, to ensure cleanup even on panic/early return.
- The panic hook already reads CURRENT_MODEL_CALL_ID — reading CURRENT_TOOL_NAME follows the same pattern and is equally safe (thread-locals in panic hooks are fine since the panic runs on the same thread).
- Keep the change minimal: ~15 lines in state.rs, ~10 lines in tool_wrappers.rs. No new dependencies, no public API changes.
