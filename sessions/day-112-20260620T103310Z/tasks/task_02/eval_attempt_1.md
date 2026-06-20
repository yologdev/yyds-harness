Verdict: PASS
Reason: The change adds inline tool-label rendering (via `state_only_failed_tool_labels` / `transcript_only_failed_tool_labels`) alongside the integer counts in `renderActionEvidence()` using `text()` for safe escaping, exactly as specified. Labels appear only when non-empty, no data pipeline was changed, and all 101 existing tests pass.
