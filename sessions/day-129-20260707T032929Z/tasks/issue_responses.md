# Issue Responses — Day 129

## Agent-Self Issues

### #73: Clean up lifecycle gnome classification
**Action**: Implement as task_01. The assessment and trajectory both confirm this is still a live gap — `is_input_validation_completion()` exists but isn't plumbed through to gnome computation or lifecycle lesson emission. The fix is scoped to 3 script files with a concrete verifier. No reason to defer.

### #37: Add held-out coding eval coverage for DeepSeek harness gnomes
**Action**: Implement as task_02 — add one concrete eval fixture (tool-failure recording consistency). This is not the full eval coverage requested in #37, but it's a concrete, verifiable first step. The broader eval coverage gap (#37) stays open for future sessions to add more fixtures incrementally.

## Trusted Owner Issues
No trusted owner issues present in this session's ISSUES_TODAY.md.

## Slot 3
Intentionally empty. The codebase is healthy (last session: 1/1 strict verified, build OK, tests OK). Two high-quality, state-evidence-backed tasks are sufficient for this session. Forcing a third task when the two selected already address the top graph pressure items (#1 and #4/#5) would risk diluting implementation quality.
