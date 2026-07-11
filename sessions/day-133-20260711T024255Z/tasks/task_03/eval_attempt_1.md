Verdict: FAIL
Reason: The `task_external_only_mismatch()` function is dead code — it has zero callers, no gnome aggregation, and the dashboard HTML was not updated to render the distinction. External-only mismatches are now silently excluded from `task_scope_mismatch_count` but never surfaced anywhere else, violating the core goal of making the distinction visible in the dashboard output.
