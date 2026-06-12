Verdict: PASS
Reason: The implementation exactly matches the task spec — a 3-line addition in `handle_why` that appends the windowing hint when `limit > 0 && events.len() >= limit`, preserving the existing error message for full-scan cases and leaving the footer hint intact. Build and tests are green.
