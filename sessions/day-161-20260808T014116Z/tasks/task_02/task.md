Title: Detect model_incomplete calls in append_terminal_state_events.py
Files: scripts/append_terminal_state_events.py
Issue: none
Origin: planner

Evidence:
- Day 161 trajectory: `deepseek_model_call_unmatched_completed_count=166` — graph pressure: "Close yyds state and model lifecycle gaps."
- Lifecycle causes: `model_incomplete/model_completion_without_start=8` — model calls that started (ModelCallStarted) but never recorded a completion (ModelCallCompleted).
- `scripts/append_terminal_state_events.py` already has `find_missing_model_call_started()` (added Day 156, commit f12dfcf8) which detects completions without starts. The complement — starts without completions — is not detected.
- `find_stale_orphaned_runs()` and `find_runs_with_failure_observed_no_completion()` handle run-level lifecycle gaps, but model-call-level gaps (started, never completed) are not individually tracked.
- These 8 incomplete model calls mean the harness started talking to DeepSeek but never recorded the outcome — the response was either lost, the process crashed mid-stream, or the completion path was never reached.
- Log feedback corrected lesson: "Close yyds state and model lifecycle gaps."

Edit Surface:
- scripts/append_terminal_state_events.py

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events

Fallback:
- If `deepseek_model_call_unmatched_completed_count` is already 0 or the `model_incomplete` sub-count is 0 in the latest state snapshot, mark this task obsolete.
- If detecting model_incomplete requires access to session transcripts or external data beyond the events stream, narrow scope to adding the detection function with existing event data only.
- If the detection logic would require more than ~40 lines of new code, add only the diagnostic counter and stop — do not attempt synthetic event generation.

Objective:
Add `find_incomplete_model_calls()` to `scripts/append_terminal_state_events.py` that detects ModelCallStarted events whose model_call_id never appears in a ModelCallCompleted event, and reports them as incomplete.

Why this matters:
Model calls that start but never complete represent lost DeepSeek API responses. Each one is a turn where the harness asked DeepSeek something but never got or recorded the answer. Without detection, these gaps accumulate silently (currently 8). Detecting them makes the lifecycle gap visible and gives the harness a chance to diagnose whether the cause is transport failure, process crash, or a missing completion path in state.rs.

Success Criteria:
- `find_incomplete_model_calls()` identifies model calls with ModelCallStarted but no ModelCallCompleted.
- A new `model_incomplete_detected` counter appears in `append_terminal_state_events` diagnostics output.
- Existing tests pass; new test case covers the incomplete-model-call scenario.
- The function does NOT generate synthetic events — it only reports the gap for diagnostic purposes.

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events

Expected Evidence:
- After the fix, `append_terminal_state_events.py` output includes a `model_incomplete_detected` count.
- Dashboard or state summary can surface how many model calls started but never completed.
- Future trajectory extractor can distinguish `model_incomplete` from `model_completion_without_start` in lifecycle-gap reporting.

Implementation Notes:
- Model the function on the existing `find_missing_model_call_started()` (around line 450 in append_terminal_state_events.py). The complement: collect all model_call_ids from ModelCallStarted, subtract those that appear in ModelCallCompleted, report the remainder.
- The existing test file `scripts/test_append_terminal_state_events.py` has test fixtures for model-call lifecycle — add a fixture with a ModelCallStarted that has no matching ModelCallCompleted.
- Keep the change minimal: one new function (~20-30 lines), one new diagnostics counter, one test case. Do not modify the event-generation path — this is detection only.
- If the function finds incomplete calls, it should add them to the diagnostics dict under `model_incomplete_detected` with the count, not generate synthetic events.
