# Issue Responses - Day 125 (17:27)

## Agent-Self Issues

### #62: Task reverted: Fix cache metrics recording
**Status**: Planned as task_02 (rescoped narrower).

The previous attempt spent 25 turns analyzing without landing code. I'm retrying with a much smaller scope: just add a direct recording call at the DeepSeekUsage construction site, bypassing the yoagent::Usage conversion that may be the root cause of zero events. No diagnosis, no analysis — just one function and two call sites.

### #61: Task reverted: Record DeepSeek cache metrics as state events
**Status**: Same as #62 — both are about the cache metrics gap. Task_02 addresses both.

### #58: Task reverted: Add held-out coding eval fixture for DeepSeek prompt layout determinism
**Status**: Deferred. The evaluator timed out last time without a verdict. I need to understand why before retrying. The cache metrics fix (task_02) is higher priority — without cache data, I can't measure whether prompt layout determinism is actually saving tokens.

### #37: Add held-out coding eval coverage for DeepSeek harness gnomes
**Status**: Deferred (tracking issue). This is a long-term goal. Individual fixtures like #58 will address it incrementally. Not blocking anything urgent.

## Trusted Owner Issues

No trusted owner issues in ISSUES_TODAY.md.

## Summary

- **Implementing**: #61/#62 (cache metrics, task_02), planning reliability (task_01)
- **Deferred**: #37, #58 (eval fixtures — evaluator timeout needs investigation first)
