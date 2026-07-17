Verdict: PASS
Reason: The diff adds --run-id flag parsing (consistent with existing --limit/--json style), filters events by run_id including "none" for orphaned events, preserves no-op behavior when flag absent, and keeps the change under 40 lines. Build and tests pass. All task success criteria are met.
