# Issue Responses — Day 133

## #93 — Task reverted: Close resolved issues #89, #91, #92
**Action:** Implement as Task 01 this session.
The previous attempt was reverted because the implementation wrote session_plan files not declared in the task's Files line. This session's Task 01 is designed to succeed: it uses gh CLI only, writes nothing to disk, and closes the three resolved issues directly.

## #37 — Add held-out coding eval coverage for DeepSeek harness gnomes
**Action:** Implement as Task 02 this session — add one concrete eval fixture.
Adding a transport error recovery fixture (037-deepseek-transport-error-recovery.json). This is incremental — one fixture per session that touches eval coverage — rather than trying to close the whole issue at once.

## #90 — Help wanted: yoagent Usage struct drops DeepSeek cache fields
**Action:** Still waiting on upstream. No replies yet. The assessment confirms `deepseek cache-report` correctly surfaces the gap. Not actionable from yyds side without a yoagent release. Leave open, no response needed this session.

## #89, #91, #92 — Resolved issues pending closure
**Action:** Will be closed by Task 01. These describe work that already shipped. Keeping them open misleads future planning sessions.
