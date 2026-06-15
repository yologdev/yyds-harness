Title: Improve cold-start `state why last-failure` with alternative diagnostic paths
Files: src/commands_state.rs
Issue: none
Origin: planner (refined from harness-seed)

Objective:
Make `yyds state why last-failure` suggest concrete alternative diagnostic
commands when no failure event exists, instead of leaving the user at a dead
end. The existing `build_why_report` already distinguishes "no sessions
completed" from "all green" from "few failures" — this task adds alternative
next-step suggestions and a branch for active/incomplete runs.

Why this matters:
The assessment's self-test of `yyds state why last-failure` returns a clean
"no state event found" because there genuinely are no failures. But a user
running this command wants diagnostic information, not just confirmation that
everything is fine. Suggesting `yyds state crashes --limit 10` or
`yyds state why last-crash` gives them an actionable next step. This also
improves the harness's observability story: when state tools are easy to
navigate, operators (and the agent itself) can diagnose problems faster.

Success Criteria:
- `state why last-failure` with no failure events suggests `state crashes`
  and `state why last-crash` as alternatives.
- `state why last-failure` with an active incomplete run mentions the run
  is still in progress.
- `state why last-crash` works as expected (it already maps to crash events).
- Existing behavior for completed failed sessions unchanged.
- Existing tests pass.

Verification:
- cargo test commands_state
- cargo check
- Manual: `yyds state why last-failure` in a clean state directory

Expected Evidence:
- Future assessments can cite concrete diagnostic suggestions instead of
  "no state event found for last-failure."
- State/dashboard operator workflows have clear escalation paths.

Implementation Notes:
The change is in `build_why_report` (line ~2571 in commands_state.rs). When
`find_target_event` returns None, the error message already has three branches
(no sessions, all green, few failures). Add:

1. In the "no sessions completed" branch: also suggest `yyds state tail --limit 5`
   to see what events exist.

2. In the "all green" branch: suggest `yyds state crashes --limit 10` and
   `yyds state why last-crash` as alternative diagnostic targets.

3. Add a new branch: check if there's a `SessionStarted` event without a
   matching `RunCompleted` (active/incomplete run). If so, mention that
   a session is still in progress and diagnostics will be available after
   it completes.

4. When `id` is specifically "last-failure" (not a custom event ID), tailor
   the suggestions to failure-specific alternatives. When `id` is a custom
   ID, keep the generic guidance.

Keep the change scoped to this one function. No new dependencies or imports
needed — all needed helpers (event_string, is_failure_event_type,
build_state_summary) are already in scope.
