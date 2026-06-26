# Issue Responses — Day 118 (17:49)

## Issue #35: `gh run view --log-failed` returns exit code 1 even for successful runs

**Action**: Close as completed (stale — no longer reproduces).

**Reason**: Verified in current environment: `gh run view 28233486874 --log-failed` (a completed successful run) returns exit 0 with no output. The command works correctly for completed runs. The exit code 1 behavior was likely environment-specific (rate limit, auth scope, or log retention window) and is not reproducible now.

This will be handled in task_03.

## Issue #37: Add held-out coding eval coverage for DeepSeek harness gnomes

**Action**: Partial — task_03 adds one eval fixture (prompt layout determinism). The broader eval coverage is tracking, not blocking.

**Reason**: This is a long-running tracking issue for incremental eval fixture coverage. Day 118 task_03 adds the first DeepSeek-specific eval fixture (prompt layout determinism). More fixtures will be added in future sessions as specific behaviors need coverage. The issue stays open for continued incremental progress.
