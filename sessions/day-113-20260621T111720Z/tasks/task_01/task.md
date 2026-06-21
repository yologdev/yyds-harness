Title: Fix stale-task detection: word-boundary match for "fail" in self-test resolution check
Files: scripts/preseed_session_plan.py
Issue: none
Origin: planner

Evidence:
- Day 113's preseed picker selected "Improve cold-start state failure diagnostics" despite the assessment's Self-Test Results showing `yyds state why last-failure → correct: reports in-progress session, incomplete run ✅`. This task was already satisfied.
- Root cause confirmed by direct test: `_self_tests_show_resolution()` returns False for a line containing "why last-failure" and ✅ because the substring check for "fail" matches "failure" in "last-failure".
- `python3 -c "from scripts.preseed_session_plan import _self_tests_show_resolution; print(_self_tests_show_resolution('why last-failure → correct ✅', ('why last-failure',)))"` → False (should be True)
- This directly causes the stale-task problem flagged as HIGH in the assessment: "task states: obsolete_already_satisfied=1"

Edit Surface:
- scripts/preseed_session_plan.py

Verifier:
- python3 -c "from scripts.preseed_session_plan import _self_tests_show_resolution; assert _self_tests_show_resolution('why last-failure → correct ✅', ('why last-failure',)); print('PASS')"

Fallback:
- If the fix doesn't resolve the assertion, narrow to word-boundary matching and re-test. If still failing, write an obsolete note explaining what else is blocking.

Objective:
Fix the stale-task selection bug in the preseed picker so that self-test lines containing "failure" (or any word containing "fail" as a substring) are not falsely excluded from resolution detection.

Why this matters:
The preseed picker's `_self_tests_show_resolution` function (line 419-429) uses a substring check for exclusion words. The word "fail" matches "failure", causing any self-test line about "state why last-failure" to be skipped. This means the picker cannot detect that cold-start diagnostics are already working, so it keeps selecting that task despite it being satisfied. This wastes ~20% of sessions (1 of the last 5).

Success Criteria:
- `_self_tests_show_resolution` returns True for lines containing "failure" when combined with a checkmark and matching task keys
- `_self_tests_show_resolution` still returns False (skips) for lines containing standalone "fail", "failed", "error", etc.
- The preseed picker's existing tests pass
- The specific regression: a self-test line "why last-failure → correct ✅" with task_keys ("why last-failure",) is detected as resolved

Verification:
- python3 -c "from scripts.preseed_session_plan import _self_tests_show_resolution; assert _self_tests_show_resolution('why last-failure → correct ✅', ('why last-failure',)); print('PASS')"
- python3 scripts/preseed_session_plan.py --test

Expected Evidence:
- Future assessment self-tests with checkmarks next to "state why last-failure" are correctly detected as resolution evidence
- The cold-start diagnostics task is no longer selected when assessment shows it working
- Stale-task count in trajectory drops

Implementation:
In `scripts/preseed_session_plan.py`, the `_self_tests_show_resolution` function at line 419-429 has:

```python
if any(word in lower for word in ("flaky", "fail", "failed", "error", "retry")):
    continue
```

The substring match for "fail" catches words like "failure". Fix: replace the substring check with word-boundary matching. Use `re.search(r'\b(?:flaky|fail|failed|error|retry)\b', lower)` instead of the `any(word in lower ...)` check. This ensures "fail" only matches the standalone word, not "failure", "failures", etc.

Note: `re.search` with `\b` word boundaries requires importing `re` at the top of the function (it's already imported at module level). Keep the logic otherwise unchanged.
