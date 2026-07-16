Title: Add retroactive ModelCallStarted for unmatched ModelCallCompleted and differentiate cancellation reasons
Files: scripts/append_terminal_state_events.py, scripts/test_append_terminal_state_events.py
Issue: none
Origin: planner

Evidence:
- Graph pressure: `deepseek_model_call_unmatched_completed_count=2` — two ModelCallCompleted events have no matching ModelCallStarted. The `append_terminal_state_events.py` retroactive pipeline handles RunCompleted, FailureObserved, and FailureObserved-without-RunCompleted gaps, but has zero handling for unmatched model call completions.
- Assessment: "Cancelled-run lifecycle: When a run is cancelled (hourly collision), a retroactive FailureObserved is correctly written, but the reason code is generic ('run completed with error status'). A more specific reason ('run cancelled by next hourly session') would improve diagnostics."
- The seed `task_01.md` was correctly flagged as stale for `state_run_incomplete` (that code already exists at line 466), but the model-call lifecycle gap (`deepseek_model_call_unmatched_completed_count`) is a separate, genuine, unaddressed problem.
- `model_lifecycle_key()` in `src/commands_state.rs:3175` matches ModelCallStarted↔ModelCallCompleted by `run_id`. When a ModelCallCompleted arrives for a run_id with no ModelCallStarted, it's unmatched.

Edit Surface:
- scripts/append_terminal_state_events.py — add `find_missing_model_call_started()`, emit retroactive ModelCallStarted, refine cancelled-run reason strings
- scripts/test_append_terminal_state_events.py — add test methods for the new retroactive path

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events

Fallback:
- If `unmatched_model_completion_count` is already 0 in a fresh `yyds state summary` (run: `cargo run --bin yyds -- state summary 2>/dev/null | grep unmatched_completed`), the gap may have self-healed. In that case, write the test to prevent regression and note the task as preventive.

Objective:
Close the model-call lifecycle gap so that every ModelCallCompleted has a matching ModelCallStarted in the event stream, and cancelled runs get precise reason strings.

Why this matters:
Unmatched model call completions distort lifecycle gnomes (`deepseek_model_call_unmatched_completed_count`), making state feedback less trustworthy for task selection. The `append_terminal_state_events.py` retroactive pipeline already handles run-level gaps — extending it to model-call-level gaps follows the same proven pattern. Differentiating cancellation reasons (hourly collision vs genuine error) makes diagnostic commands like `yyds state why last-failure` more informative.

Success Criteria:
- `find_missing_model_call_started()` scans events for ModelCallCompleted entries whose `run_id` has no prior ModelCallStarted, returning a list of (run_id, model, timestamp_ms).
- For each unmatched completion, a retroactive ModelCallStarted is emitted with `"retroactive": true` in the payload and `model_call_id: "retroactive-<run_id>"`.
- Cancelled runs (status "cancelled" in RunCompleted) get a FailureObserved reason like "run cancelled by next hourly session" instead of the generic "run completed with error status 'error'".
- Existing tests in `test_append_terminal_state_events.py` continue to pass.

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events

Expected Evidence:
- Future `yyds state summary` shows `deepseek_model_call_unmatched_completed_count` = 0.
- `yyds state why last-failure` for cancelled runs shows a specific cancellation reason.
- Test coverage for the new retroactive ModelCallStarted path.

Implementation Notes:
- Follow the existing pattern in `append_terminal_state_events.py`: add a new `find_*()` function, call it in `append_terminal_events()`, emit retroactive events through `_maybe_append_event()`.
- The `model_lifecycle_key` in `src/commands_state.rs` uses `run_id` to match pairs. So the retroactive ModelCallStarted should use the same `run_id` as the orphaned ModelCallCompleted, with `model_call_id` set to `"retroactive-<run_id>"` and `model` extracted from the ModelCallCompleted's payload.
- For cancelled-run differentiation: check if the RunCompleted payload's `status` field is `"cancelled"` (as opposed to `"error"` or `"success"`). The current `find_missing_failure_observed` scans for error-status completions; add a cancellation-specific reason when status is `"cancelled"`.
- Do not modify `scripts/evolve.sh` (protected file).
