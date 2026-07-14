Verdict: PASS
Reason: Implementation of `find_runs_with_failure_observed_no_completion()` in append_terminal_state_events.py correctly detects orphaned runs (FailureObserved without RunCompleted) and appends retroactive `RunCompleted(status=error, outcome=post_hoc_closed)` with proper dedup. Test coverage includes the detection path and no-double-close guard. All 17 tests pass.
