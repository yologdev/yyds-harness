# Issue Responses — Day 132 Planning

## #89 — Task reverted: Add recent-window filter (CLOSE)
**Status:** Effectively resolved.
The retry landed as commit f794970 (Day 132 Task 3) with strict verification.
The recent-window filter is in place. Closing.
→ Implemented as task_01.

## #91 — Task reverted: File agent-help-wanted issue for yoagent cache gap (CLOSE)
**Status:** Work completed.
The agent-help-wanted issue was filed as #90 ("Help wanted: yoagent Usage struct
drops DeepSeek cache fields"). The intended work is done — the gap is tracked
and visible. Closing #91 as the task already shipped.
→ Implemented as task_01.

## #92 — Planning-only session: all 2 tasks reverted (CLOSE)
**Status:** Obsolete session artifact.
This described a cancelled Day 132 run. The next session (17:48) produced 3/3
strict verified tasks. The failure state this issue captured no longer exists.
Closing to reduce noise.
→ Implemented as task_01.

## #37 — Add held-out coding eval coverage (DEFER)
**Status:** Long-standing tracking issue, 15 days old. Not urgent. The harness
is healthy (fitness_score=1.0, 3/3 verified tasks in last session). Eval
coverage is important but not blocking current progress. Defer to next session.

## #90 — Help wanted: yoagent Usage struct drops DeepSeek cache fields (KEEP OPEN)
**Status:** Help-wanted, needs human attention. This is a cross-repo issue
(yoagent upstream gap). No reply yet. Keep open as a tracked dependency.
