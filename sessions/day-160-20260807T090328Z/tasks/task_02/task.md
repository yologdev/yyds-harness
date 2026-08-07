Title: Add input-validation and cancelled-run exclusion to find_runs_with_failure_observed_no_completion
Files: scripts/append_terminal_state_events.py
Issue: none
Origin: planner

Evidence:
- Trajectory Day 160 graph pressure: `state_unmatched/open_after_FailureObserved=8` — the top lifecycle gap metric.
- `find_missing_failure_observed()` (line 307) was recently hardened with three exclusions: input-validation runs, cancelled runs, and deliberate no-op runs (committed bad5fbd5, Day 160).
- `find_runs_with_failure_observed_no_completion()` (line 409) detects runs with FailureObserved but no RunCompleted — it has NONE of these exclusions. It is inconsistent with its sibling function.
- #162 was reverted for scope mismatch — implementation touched wrong files (.skill_evolve_counter, memory/learnings.jsonl instead of the planned scripts). This task narrows scope to one file.
- The `collect_input_validation_run_ids()` helper (line 445) already exists and can be reused.

Edit Surface:
- scripts/append_terminal_state_events.py

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events

Fallback:
- If `open_after_FailureObserved` is already 0 in the latest state snapshot OR if the function already has exclusion logic (from a parallel fix), mark this task obsolete.
- If excluding input-validation runs causes existing test failures due to fixture mismatch, keep the exclusion but update the diagnostic counter only (skip the actual filtering).

Objective:
Make `find_runs_with_failure_observed_no_completion` consistent with `find_missing_failure_observed` by adding the same input-validation and cancelled-run exclusions. This prevents future false lifecycle-gap alarms and makes the two sibling functions maintainable as a pair.

Why this matters:
The `find_missing_failure_observed` function was hardened in Day 160 with three exclusion categories. Its sibling `find_runs_with_failure_observed_no_completion` — which detects the complementary lifecycle gap (FailureObserved without RunCompleted) — was not updated. While input-validation runs currently shouldn't have FailureObserved events, defensive consistency prevents a future code change from silently introducing false positives. The two functions should evolve together.

Success Criteria:
- `find_runs_with_failure_observed_no_completion` skips runs whose RunCompleted payload indicates input-validation (empty_input or invalid_input:*).
- `find_runs_with_failure_observed_no_completion` skips runs whose RunCompleted status is "cancelled".
- The diagnostics dict includes `input_validation_excluded` and `cancelled_excluded` counters (matching `find_missing_failure_observed`).
- Existing tests pass. New test cases cover input-validation and cancelled-run exclusion.

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events
- python3 scripts/append_terminal_state_events.py --help (confirm no regressions)

Expected Evidence:
- `failure_observed_no_completion_diagnostics` in append_terminal_state_events output includes the new exclusion counters.
- Future state snapshots maintain or reduce `open_after_FailureObserved` — false positives from validation/cancelled runs are blocked.
- The two sibling functions (`find_missing_failure_observed` and `find_runs_with_failure_observed_no_completion`) share the same exclusion categories.

Implementation Notes:
- The change is in `find_runs_with_failure_observed_no_completion()` at line 409.
- The function currently iterates events to build `failure_observed_runs` and `run_completed_runs` sets. Add a third set `excluded_runs` populated from RunCompleted events with input-validation detail or cancelled status.
- Subtract `excluded_runs` from `missing` before returning.
- Add `input_validation_excluded` and `cancelled_excluded` counters to the diagnostics dict.
- Reuse the same detection logic as `find_missing_failure_observed` (lines 350-356): check error_detail for "empty_input"/"invalid_input:" prefix, and status == "cancelled".
- Keep the change minimal — do not refactor the event loop or add new helper functions.
- The existing test data may not have input-validation runs with FailureObserved — that's OK, the exclusion is defensive. Add a test that constructs a synthetic event list with an input-validation RunCompleted + a FailureObserved for the same run_id and verifies the run is NOT in the returned missing set.
