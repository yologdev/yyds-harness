Verdict: PASS
Reason: The diff adds `recent_state_only_failed_tool_count` and `recent_transcript_only_failed_tool_count` keys to `action_evidence_summary_for_sessions()` via a new `_recent_failed_tool_total()` helper using the last 5 sessions, exactly matching the task spec. The change is additive, minimal, and build/tests are PASS.
