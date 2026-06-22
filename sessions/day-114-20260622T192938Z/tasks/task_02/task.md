Title: Close transcript-only tool-failure state-capture gap
Files: src/tool_wrappers.rs, src/state.rs
Issue: none
Origin: planner

Evidence:
- Assessment Day 114 bug list: "MEDIUM — Transcript-only tool failure gap: One tool failure visible in transcripts but absent from state events (transcript_only_failed_tool_count=1). This is a state-capture completeness issue — the events pipeline isn't recording every tool failure that the transcript captures."
- Trajectory structured state snapshot: "recent action evidence: state_only_failed_tools=34, transcript_only_failed_tools=1"
- Dashboard detection in scripts/build_evolution_dashboard.py:2671 computes transcript_only_failed_tool_count by set-diffing transcript-failed tools against state-failed tools.

Edit Surface:
- src/tool_wrappers.rs — ToolFailureTracker and its integration with state recording
- src/state.rs — StateRecorder event persistence

Verifier:
- cargo test -- tool_failure
- cargo test -- state

Fallback:
- If investigation confirms the 1 transcript-only failure is a dashboard false positive (not a real state gap), document the finding and mark the task verified without code changes. If the gap is real but requires >3 files to fix, narrow the task to a diagnostic improvement that makes the gap visible in state why or state doctor output.

Objective:
Ensure every tool failure visible in agent transcripts is also recorded as a state event, so dashboard claims and state-based diagnostics accurately reflect real failure counts.

Why this matters:
When tool failures are visible in transcripts but absent from state events, the dashboard undercounts real failures. This makes state why, state doctor, and evolution dashboard claims less trustworthy. A 1-instance gap today could be a systematic blind spot that grows — finding and closing the specific failure path now prevents silent degradation of state integrity.

Success Criteria:
- transcript_only_failed_tool_count drops to 0 in the next session where tool failures occur
- OR: investigation proves the single instance was a dashboard false positive and the state pipeline is complete

Verification:
- cargo test -- tool_failure
- cargo test -- state
- grep for ToolFailureTracker usage to confirm all tool-call error paths are wrapped
- If a code fix is made, add a targeted unit test for the specific failure path that was missing state recording

Expected Evidence:
- Future dashboard runs show transcript_only_failed_tool_count=0 when state capture is complete
- Task lineage shows a targeted fix in src/tool_wrappers.rs or src/state.rs with passing tests

Implementation Notes:
- The gap is exactly 1 instance — a specific edge case, not a broad failure. Start by understanding how ToolFailureTracker records failures and where the recording pipeline could drop one.
- ToolFailureTracker in src/tool_wrappers.rs wraps tool calls and tracks failures. Check whether every error exit path calls the state recorder. Look for early returns, error branches, or async paths that might bypass recording.
- In src/state.rs, check whether the StateRecorder's record method can silently drop events (e.g. if the global recorder isn't initialized, or if certain event types are filtered).
- The dashboard detection (scripts/build_evolution_dashboard.py, read-only) computes the gap by comparing two sets. A false positive is possible if transcript parsing is looser than state event parsing — but the assessment treats this as a real state gap, so prefer finding the actual missing recording path.
- Do NOT modify scripts/build_evolution_dashboard.py — the dashboard is reporting a real signal; fix the signal source, not the reporter.
- If you find that ToolFailureTracker is correctly recording but the state persistence layer drops the event, add a test that verifies event round-trip through the recorder.
