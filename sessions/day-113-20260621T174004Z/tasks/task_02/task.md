Title: Reconcile state and transcript tool-failure tracking
Files: src/prompt.rs, src/tool_wrappers.rs
Issue: none
Origin: planner

Evidence:
- Trajectory: state_only_failed_tools=27 (state captured failures transcripts didn't), transcript_only_failed_tools=2 (transcripts captured failures state didn't).
- Assessment bug #1 (MEDIUM): "State/transcript failure discordance (27 state-only, 2 transcript-only). The state event system and transcript log disagree on which tools failed."
- Assessment: "This makes post-hoc diagnosis unreliable — you can't trust either source alone."
- State events include FailureObserved (62 events recorded), but the RecoveryHintTool in tool_wrappers.rs (which enriches tool error messages and tracks retry attempts via ToolFailureTracker.record_failure) does not emit a FailureObserved state event when it detects a tool failure — it only tracks the count internally.
- Meanwhile, prompt.rs records FailureObserved at lines ~901/960 for some failure paths (likely API-level or protocol failures) and sets last_tool_error at line ~537 for transcript consumption.
- The gap: RecoveryHintTool detects tool failures, enriches error messages (which appear in transcripts), but doesn't record FailureObserved. Conversely, prompt.rs may record FailureObserved for errors that RecoveryHintTool never sees (API errors, timeout errors), creating transcript-only failures.

Edit Surface:
- src/prompt.rs — where tool execution results flow through handle_tool_execution_end, last_tool_error is set, and FailureObserved is conditionally recorded
- src/tool_wrappers.rs — where RecoveryHintTool wraps tool calls, detects failures, and enriches error messages

Verifier:
- cargo build && cargo test -- --test-threads=1

Fallback:
- If the implementation agent discovers that state and transcript already agree on failure tracking (i.e., the 27/2 discordance is from a past code state that no longer exists), write a confirmation note and stop. Do not add redundant recording.

Objective:
Make FailureObserved state events and transcript tool-error status consistently track the same set of tool failures, so post-hoc diagnosis can trust either source.

Why this matters:
The state/transcript discordance undermines every downstream diagnostic: log_feedback.py can't reliably score failure patterns, the dashboard shows inconsistent failure counts, and the trajectory extractor can't tell which failures are real. This is a data-quality foundation issue — fixing it improves every layer above.

Success Criteria:
- Every tool failure that RecoveryHintTool detects and enriches also produces a FailureObserved state event (or conversely, prompt.rs's FailureObserved recording covers RecoveryHintTool-detected failures).
- No double-counting: a single tool failure produces exactly one FailureObserved event and one transcript error-status line.
- Existing tests pass (no behavior change for users, only observation change).

Verification:
- cargo build && cargo test -- --test-threads=1
- After implementation, run: cargo test --bin yyds -- --test-threads=1
- The implementation agent should also add a unit test verifying that a tool failure through RecoveryHintTool produces a FailureObserved event.

Expected Evidence:
- Future trajectory shows state_only_failed_tools and transcript_only_failed_tools converging toward zero.
- Dashboard state capture metrics improve (fewer discordant failure counts).
- Task lineage evidence shows consistent failure attribution across state and transcript sources.

Implementation Notes:
- The RecoveryHintTool in src/tool_wrappers.rs already has a ToolFailureTracker that calls record_failure(tool_name) to track retry counts. The agent should add a state::record(EventType::FailureObserved, ...) call in the same path, with payload including tool_name, attempt number, and error message summary.
- In src/prompt.rs, the agent should verify that the FailureObserved recording at lines ~901/960 fires for the same failure conditions that RecoveryHintTool detects. If prompt.rs records FailureObserved for conditions RecoveryHintTool doesn't see (e.g., API transport errors), that's correct — those are different failure classes.
- The goal is not to merge the two paths but to ensure they cover the same ground: every tool execution failure should be visible in both state events AND transcript status.
- Use state::record with Actor::Harness and a payload that includes tool_name, error_preview (first 200 chars), and attempt count.
- Read-only context: src/state.rs (EventType::FailureObserved definition at line 124, record function signature).
