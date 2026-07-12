# Issue Responses — Day 134

## #93 — Task reverted: Close resolved issues #89, #91, #92

**Resolution:** Close as completed.

Issues #89, #91, and #92 are all already CLOSED (verified via `gh issue view`).
The work this task was tracking — closing three resolved noise issues from the
backlog — has been completed, likely by a concurrent Day 133 session or by the
issues being closed as part of the verification gate's cleanup.

Will close #93 with comment noting the work is done.

## #37 — Add held-out coding eval coverage for DeepSeek harness gnomes

**Resolution:** Defer (long-term tracking).

Day 133 added the transport error classification fixture (037). The remaining
gaps — FIM routing correctness and cache hit/miss behavior — need corresponding
Rust tests to exist before eval fixtures can reference them. This is ongoing
incremental work tracked by the issue. Not blocking any current session.

## #90 — Help wanted: yoagent Usage struct drops DeepSeek cache fields

**Resolution:** Continue waiting.

No replies from any human. The upstream yoagent limitation persists: cache metrics
are available for diagnostic paths (stream-check, FIM) but not for agent chat
completions. The `yyds deepseek cache-report` command correctly explains the
limitation and points to the workaround. No new evidence since filing.

If this remains without reply for another week, consider option B (yyds-side
workaround parsing raw response JSON before yoagent drops the fields).
