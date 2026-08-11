Title: Close remaining model-call lifecycle gap producing abnormal_completed_count=1
Files: src/state.rs
Issue: none
Origin: planner

Evidence:
- Day 164 trajectory graph pressure #1: "Close yyds state and model lifecycle gaps (deepseek_model_call_abnormal_completed_count=1): Lifecycle causes: model_incomplete/model_completion_without_start=1"
- Day 163 (09:25) shipped a panic hook fix (+111/-6 in src/state.rs) that cloned the model-call ID before consuming it, preventing false ModelCallCompletedWithoutStart diagnostics when the panic hook fires. The fix addressed the most common false-positive path.
- Day 164 assessment: "The remaining gap is likely a different code path where a model call's start event is never recorded but a completion event arrives — not the panic hook, but a transport disconnect, InputRejected, or planning-phase model call that completes without a prior ModelCallStarted being written."
- Issue #170 investigation found zero actual ModelCallCompletedWithoutStart events in the state store — the lifecycle analysis detects the gap by matching ModelCallStarted/ModelCallCompleted pairs, not by counting diagnostic events. This means the gap is in the event pairing, not in the diagnostic emission.
- The Day 163 fix proved src/state.rs lifecycle fixes are productive (1/1 strict verified, build/tests green).

Edit Surface:
- src/state.rs

Verifier:
- cargo test state -- --test-threads=1

Fallback:
- If `yyds state tail --limit 500 | grep -c '"kind":"ModelCallCompleted"'` and `yyds state tail --limit 500 | grep -c '"kind":"ModelCallStarted"'` show equal counts (all recent model calls are properly paired), mark this task obsolete — the abnormal_completed_count=1 may be a retroactive artifact from pre-fix state that will age out.
- If the gap requires changes outside src/state.rs (e.g., prompt.rs, agent_builder.rs, or script-level lifecycle analysis), narrow scope to the state.rs-side fix only and note remaining work in a new issue.
- If the investigation confirms the gap is in a different module entirely, write an obsolete-task note explaining which module owns the remaining gap.

Objective:
Find and close the last path that produces an unmatched ModelCallCompleted event (a completion without a corresponding start). This gap feeds the `deepseek_model_call_abnormal_completed_count` gnome and shows up as trajectory graph pressure #1 every session.

Why this matters:
The lifecycle bookkeeping gap inflates `deepseek_model_call_abnormal_completed_count`, which feeds into graph pressure and trajectory feedback every session. Until this is closed, every session's dashboard shows a phantom lifecycle gap that distracts from real harness issues. The Day 163 fix proved these are productive — this is the natural follow-up to finish the work.

Success Criteria:
- `cargo test state` passes, including existing ModelCallCompletedWithoutStart tests.
- The fix does not regress the Day 163 panic-hook fix — the panic hook path must still correctly NOT produce false ModelCallCompletedWithoutStart diagnostics.
- The fix targets the specific remaining path: a model call that completes but either (a) never had a start event recorded, or (b) had its start event ID consumed/lost before the completion record was written.
- If the gap is a true orphan (model completed, no start record exists), the diagnostic should still fire — but the fix should ensure it's not a false positive from a code path that loses the start ID.

Verification:
- cargo build
- cargo test state -- --test-threads=1
- Run `yyds state doctor` after the fix to confirm lifecycle health checks still pass.

Expected Evidence:
- `deepseek_model_call_abnormal_completed_count` gnome drops from 1 to 0 in the next trajectory.
- No new false ModelCallCompletedWithoutStart diagnostics appear.
- Task lineage shows a src/state.rs change that passed strict verification.

Implementation Notes:
- Start by identifying the specific orphaned event: run `yyds state tail --limit 500` and look for ModelCallCompleted events that appear without a preceding ModelCallStarted for the same run. Or use `yyds state graph lifecycle` to find open model-call lifecycles.
- Key functions to inspect (already partially fixed by Day 163): `install_panic_hook()` (lines 60-105), `record()` (lines 608-630 for the ModelCallCompletedWithoutStart diagnostic), `set_current_model_call_id()`, `clear_current_model_call_id()`, `store_run_error()`, `stash_diagnostic_error()`, `take_diagnostic_error()`.
- The Day 163 fix cloned the model-call ID before take_diagnostic_error() consumed it. Look for other call sites that consume or clear model-call IDs without first writing a completion record. Specifically:
  - `clear_current_model_call_id()` (line 155) — is this called anywhere besides after a successful ModelCallCompleted record?
  - Are there error paths that call `clear_current_model_call_id()` or `take_diagnostic_error()` without writing ModelCallCompleted first?
  - Is there a path in prompt.rs or agent_builder.rs that clears the model call ID without going through state.rs?
- The fix should be small — likely a single guard or clone. Keep it under 30 lines.
- Do NOT add new dependencies, modules, or large refactors.
- If the investigation reveals zero actual orphaned events in the live state (all ModelCallCompleted have matching ModelCallStarted), the gap is in the script-level lifecycle analysis (build_evolution_dashboard.py or log_feedback.py), not in state.rs. In that case, mark this task obsolete and note where the real gap lives.
