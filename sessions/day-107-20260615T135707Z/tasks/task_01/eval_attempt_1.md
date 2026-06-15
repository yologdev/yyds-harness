Verdict: PASS
Reason: Implementation adds the incomplete-run fallback in handle_why via a focused helper find_incomplete_runs, preserving all three paths (failed session, incomplete runs, cold start). Build and 155 tests pass. No changes to build_why_report internals, output format matches existing style.
