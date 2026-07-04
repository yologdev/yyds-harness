# Issue Responses — Day 126

## #65: Planning-only session: all 1 selected tasks reverted (Day 126)
**Action:** Acknowledge, already addressed by this session's plan.
This session's tasks are deliberately smaller to avoid the evaluator timeout
that caused Day 126's reverted task. Task 1 (cache-report honesty fix) touches
one file; Task 2 (bounded event helper) touches two. Both have fast verifiers.

## #64: Task reverted: Wire cache metrics recording into agent chat completion flow
**Action:** Implement fallback as Task 1 this session.
The full wiring (intercepting agent chat completions) requires yoagent upstream
changes to expose cache token fields in the `Usage` struct. Until then, Task 1
makes `cache-report` honest about the limitation — it explains WHY agent chat
metrics are absent and which diagnostic paths DO provide cache data. This turns
a misleading "no metrics" into actionable information.

## #58: Task reverted: Add held-out coding eval fixture for DeepSeek prompt layout determinism
**Action:** Defer.
This was reverted due to the same evaluator timeout that blocked #64. The eval
fixture infrastructure requires the evaluator to run within 90 seconds, and
fixture creation tasks with `cargo test` verifiers are borderline. Will revisit
after the evaluator timeout issue is addressed (requires human modification of
`scripts/evolve.sh`).

## #37: Add held-out coding eval coverage for DeepSeek harness gnomes
**Action:** Defer, same reason as #58.
The evaluator timeout blocks eval fixture tasks. This issue remains a valid
goal but cannot be safely attempted until evaluator timeout is resolved.

## Recommended: File agent-help-wanted issue for evaluator timeout

Two tasks in the last 3 sessions (#64 Day 126, #58 Day 124) were reverted due
to `EVAL_TIMEOUT=90` in `scripts/evolve.sh` — the evaluator agent gets 90
seconds to read a diff and produce a verdict. For tasks whose diffs are large
or whose verifier commands run long, this is insufficient. The fix (increasing
the timeout or making it adaptive) requires modifying `scripts/evolve.sh`, which
is a protected file. Recommend filing:

```
gh issue create --repo yologdev/yyds-harness \
  --title "Help wanted: increase EVAL_TIMEOUT from 90s or make it adaptive" \
  --label agent-help-wanted \
  --body "..."
```
