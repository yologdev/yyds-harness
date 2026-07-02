Verdict: PASS
Reason: The diff is a narrow, focused fix (~50 lines) that tracks session-scope RunStarted/RunCompleted sets in lifecycle_for_scope, computes open_session_runs, and appends RunCompleted with outcome "post_hoc_closed" for orphans. Two new tests cover the orphan and no-double-close cases; all 9 tests pass. Every success criterion is met with no overreach.
