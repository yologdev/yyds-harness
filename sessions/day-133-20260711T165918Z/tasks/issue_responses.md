# Issue Responses — Day 133 Planning

## Agent-Self Issues

### #93: Task reverted: Close resolved issues #89, #91, #92
**Plan:** Implement as task_02. The verification gate (`scripts/task_verification_gate.py`)
is the blocker — its `external_only_planned()` function is too narrow and doesn't
recognize non-code tasks. Task 02 broadens the detection so this task can pass
verification on retry.

### #37: Add held-out coding eval coverage for DeepSeek harness gnomes
**Plan:** Defer. Progress was made Day 133 02:42 (transport error fixture added).
This session's three slots are filled with higher-impact work: planning reliability
(task_01), verification gate fix (task_02), and subcommand help UX (task_03).
Eval coverage is incremental tracking — each session that has spare capacity
should add one fixture. Continue next session.

## Help-Wanted

### #90: yoagent Usage struct drops DeepSeek cache fields
**Plan:** Defer. No upstream yoagent repo is configured and no human has replied.
This is blocked until either (a) a human adds the fields to yoagent's Usage struct
upstream, or (b) we implement the yyds-side workaround (Option B: parse raw
response JSON before yoagent drops the fields). Neither fits in this session's
budget. The assessment confirms: "Issue #90 is filed as agent-help-wanted for
yoagent. No action needed here — it's tracked."

## Trusted Owner Issues

None in ISSUES_TODAY.md requiring action this session.
