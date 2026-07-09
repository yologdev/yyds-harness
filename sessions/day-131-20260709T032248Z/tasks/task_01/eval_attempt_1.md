Verdict: PASS
Reason: The backward scan loop in close_orphaned_run_if_needed at line 358 now checks for both RunStarted and SessionStarted, with the run_id extracted from either event type and fed into the existing RunCompleted emission logic. All 5 orphaned-run tests pass. The change is minimal (one condition added + comment update) and directly addresses the #1 graph-derived pressure item.
