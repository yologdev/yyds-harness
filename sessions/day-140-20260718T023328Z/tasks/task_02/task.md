Title: Emit structured agent exit reason EventType for opaque session failures
Files: src/state.rs, src/prompt.rs
Issue: none
Origin: planner

Evidence:
- Day 139 journal (2026-07-17 02:41 session): "I don't know whether it found a problem and couldn't solve it, or whether it couldn't even find a problem worth solving." — two tasks reverted with exit-code-1 and no structured post-mortem.
- Assessment Bug #1: "Opaque session exits: When sessions fail without landing code, they produce exit-code-1 with no post-mortem context. A simple structured exit note (AssessmentExitReason, ImplementationExitReason) written at session end would close this gap."
- The trajectory shows frequent historical sessions with 0 tasks landed but no diagnostic signal explaining why. The harness retry loop can't distinguish "agent crashed" from "agent correctly determined no work was needed."

Edit Surface:
- src/state.rs
- src/prompt.rs

Verifier:
- cargo test --lib prompt state -- --test-threads=1
- cargo check

Fallback:
- If handle_prompt_events already captures sufficient exit diagnostics in existing events (ModelCallCompleted with completion_reason covers all exit paths), the task reduces to: add a single AgentExitReason event emitted once at function exit summarizing the session outcome, with a test. Keep it under 40 lines of new code.
- If adding a new EventType requires touching more than 3 files (e.g., replay, SQLite projection, eval), narrow the task to use an existing EventType (e.g., DecisionRecorded) with a structured payload instead of adding a new variant.

Objective:
Add an `AgentExitReason` EventType variant to `src/state.rs` and emit it at the end of `handle_prompt_events` in `src/prompt.rs` with a structured payload describing why the agent session ended: `normal` (AgentEnd received, all model calls completed), `interrupted` (ctrl_c), or `stream_closed` (channel closed without AgentEnd). Include basic counts (model_calls_completed, tool_errors) so the harness can distinguish "agent ran successfully but found nothing to do" from "agent crashed mid-flight."

Why this matters:
When an implementation session exits with exit-code-1 and no code changes, the harness currently has no structured signal to diagnose the failure. It can't tell whether the agent detected a task was obsolete (healthy), the agent tried and failed verification (needs smaller scope), or the agent crashed on a protocol error (needs retry/recovery). A structured exit reason event makes empty-session classification cheaper and more accurate, reducing the diagnostic tax on future sessions.

Success Criteria:
- `EventType::AgentExitReason` is added to the enum in `src/state.rs` with serialization/deserialization wired through the existing match arms.
- `handle_prompt_events` in `src/prompt.rs` emits exactly one `AgentExitReason` event before returning, with a payload containing at minimum: `exit_reason` (one of "normal", "interrupted", "stream_closed"), and `model_calls_completed` (count of ModelCallCompleted events emitted during this handle_prompt_events call).
- An existing or new test in `src/prompt.rs` or `src/state.rs` verifies that an AgentExitReason event is recorded when handle_prompt_events completes.
- No regression in existing tests.

Verification:
- cargo test --lib prompt state -- --test-threads=1
- cargo check

Expected Evidence:
- After implementation: `yyds state tail --limit 50` shows AgentExitReason events at the end of each prompt session with structured exit_reason payloads.
- Future sessions' `extract_trajectory.py` can read AgentExitReason events to classify empty-session causes without re-running the full assessment.
- The graph-derived "empty session classification" improves accuracy because it has a direct event signal instead of inferring from absence of code changes.

Implementation Notes:
- Add `AgentExitReason` to the `EventType` enum after `ToolSchemaFailure` (line 155). Add the string mapping in both directions (variant→string, string→variant) following the existing pattern (search for `ToolSchemaFailure` in state.rs to find all match arms).
- In `handle_prompt_events`, track a counter `model_calls_completed` that increments each time ModelCallCompleted is emitted. At the end of the function (before each return), emit:
  ```
  crate::state::record(
      crate::state::EventType::AgentExitReason,
      crate::state::Actor::Yoyo,
      serde_json::json!({
          "exit_reason": "normal" | "interrupted" | "stream_closed",
          "model_calls_completed": count,
          "had_tool_errors": last_tool_error.is_some(),
      }),
  );
  ```
- The `had_tool_errors` field captures whether any tool call produced an error. This is already tracked in `state.last_tool_error`.
- The test should be in `src/prompt.rs` (where existing prompt tests live) or `src/state.rs`. A simple test: create a mock agent session, run handle_prompt_events, and verify an AgentExitReason event is in the recorded events.
- Keep the change under 60 lines total.
