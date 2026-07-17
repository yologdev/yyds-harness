# Issue Responses — Day 139

## #111 — Task reverted: Deduplicate retroactive FailureObserved events
**Plan:** Implement as task_01 this session.
The last attempt failed because the implementation touched only the test file
when the planned file was the main script. This time both files are in scope.
The fix is small and well-defined: add a test for multi-invocation dedup, and
if the code doesn't already handle it, add a retroactive-event check to
`find_missing_failure_observed`.

## #105 — Task reverted: Record DeepSeek prompt cache metrics
**Plan:** Defer.
This is blocked on upstream yoagent issue #90 — the `Usage` struct doesn't
expose `cache_read_input_tokens` and `cache_creation_input_tokens`. Without
those fields, there's nothing to record from agent chat completions. The
implementation agent burned 24 turns last time reading code without being able
to land anything because the data simply isn't available through yoagent's API
surface. We need the upstream fix before this task can succeed.

## #112 — Planning-only session: all 2 selected tasks reverted (Day 139)
**Plan:** Close after this session if task_01 lands.
This is the meta-issue tracking the Day 139 02:42 session's 0/2 outcome. If
task_01 (#111) lands this session, the root cause (scope_mismatch on a
well-scoped task) is addressed and this issue can be closed.

## #90 — Help wanted: yoagent Usage struct drops DeepSeek cache fields
**Plan:** No action this session.
Still waiting for upstream or a human to decide between Option A (yoagent PR)
and Option B (yyds-side workaround). No reply yet.
