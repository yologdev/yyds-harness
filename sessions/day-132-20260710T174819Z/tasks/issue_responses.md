# Issue Responses — Day 132 (17:48)

## #89 — Task reverted: Add recent-window filter to state/transcript tool failure reconciliation
**Action**: Implement as task_03 (smaller scope — dashboard data source only, no trajectory renderer change)

The original task was reverted due to evaluator timeout, not code failure. The premise is still valid: `state_only_failed_tool_count=37` includes historical cascade noise that makes the signal non-actionable. This retry scopes down to just adding the recent-window computation to `action_evidence_summary_for_sessions()` in `build_evolution_dashboard.py`. The trajectory renderer half is deferred to a follow-up.

## #91 — Task reverted: File agent-help-wanted issue for yoagent DeepSeek cache field gap
**Action**: Defer — the help-wanted issue (#90) has already been filed manually

Issue #90 already exists with the full problem description, evidence, and resolution paths. Filing a duplicate would be noise. The verification gate correctly caught that issue creation produces no git-visible changes — this task class needs a different verification approach. Until we have a way to verify issue-creation tasks, this stays deferred.

## #92 — Planning-only session: all 2 selected tasks reverted (Day 132)
**Action**: Close after this session lands

This was a meta issue tracking the session that reverted #89 and #91. If this session lands task_02 and task_03 (the `state why` timeout fix and the #89 retry), the "no code shipped" problem is resolved. Will close after verification.

## #37 — Add held-out coding eval coverage for DeepSeek harness gnomes
**Action**: Defer — lower priority than concrete bug fixes

This is a long-standing tracking issue (June 25). Current sessions are landing code successfully (1/1, 2/2 strict verified in recent runs), so eval coverage is a nice-to-have, not a blocker. The `state why` timeout bug and tool-failure signal noise are more urgent — they directly block diagnostic workflows.

## #90 — Help wanted: yoagent Usage struct drops DeepSeek cache fields
**Action**: Wait for human reply

Already filed with full evidence. No replies yet. This requires either upstream yoagent changes or a yyds-side workaround — both need human input on approach.
