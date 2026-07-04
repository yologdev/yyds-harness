Verdict: FAIL
Reason: Implementation breaks 2 existing tests (test_closes_open_model_and_run_after_line, test_ambiguous_reset_scan_does_not_close_historical_open_runs) because the new orphan detector scans the entire file and closes runs that tests expect to remain open. No new test for orphan detection was added. The task's success criteria of "no regression" and "new test passes" are unmet.
