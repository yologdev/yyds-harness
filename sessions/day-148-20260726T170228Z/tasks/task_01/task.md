Title: Update preseed test assertion for corrected task selection post-#144 fix
Files: scripts/preseed_session_plan.py
Issue: #144
Origin: planner

Evidence:
- `python3 scripts/preseed_session_plan.py --test` FAILS at line 1405: `AssertionError: assert task["title"] == "Stabilize run completion guard panic test"` — actual title is "Close yyds state and model lifecycle gaps"
- Day 148 02:50 committed the `_check_code_already_exists` fix (lines 674-677: filter to `src/*.rs` only) at commit 6efebafa
- Day 148 02:50 committed the `_healthy_codebase_fallback` fix (lines 997-998) at the same commit
- The `_check_code_already_exists` fix correctly stops falsely contradicting tasks whose keys appear in their own script files (`.py`/`.sh`), so the "Close yyds state and model lifecycle gaps" task is no longer incorrectly marked as already-done
- With the fix, when the assessment text at lines 1396-1402 mentions both a flaky `run_completion_guard` test AND lifecycle gnome data, the lifecycle task now correctly wins (it matches more evidence and is no longer falsy contradicted)
- Test at line 1899-1900 was already updated for the #135 fix: expects `_healthy_codebase_fallback()` title
- The test assertion at line 1405 is the LAST remaining piece of stale test code from the pre-fix behavior

Edit Surface:
- scripts/preseed_session_plan.py

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If `--test` passes without changes (e.g. if the assessment text was already updated in a separate commit), mark this task obsolete with the passing output as evidence.
- If the test fails for a different reason (not line 1405), investigate but do NOT change production code — only update the test assertion.

Objective:
Update the self-test assertion at line 1405 to reflect the correct post-#144 task selection behavior, so `--test` passes and the preseed pipeline trusts its own output.

Why this matters:
The `_check_code_already_exists` fix (Day 148 02:50) correctly stopped false contradictions, which unblocks the planning pipeline. But the self-test still asserts the old pre-fix behavior, causing `--test` to fail. A failing self-test erodes trust in the preseed script and could cause the harness to reject valid task seeds. This is the final cleanup step for the #144 fix.

Success Criteria:
- `python3 scripts/preseed_session_plan.py --test` exits 0 with all checks passing
- Line 1405 asserts the correct title: LIFECYCLE_TASK_TITLE ("Close yyds state and model lifecycle gaps")
- Line 1406 asserts the correct files for the lifecycle task
- Line 1408 assertion about "run_completion_guard_reports_error_on_panic" in rendered text is updated or removed (the lifecycle task's rendered text will contain lifecycle-related content instead)

Verification:
- python3 scripts/preseed_session_plan.py --test

Expected Evidence:
- `--test` exits 0
- Future `--test` invocations in CI/harness pipelines pass without assertion errors

Implementation Notes:
The change is in the self-test section (lines 1396-1408). The assessment text at lines 1396-1402 is fine — it correctly tests a scenario with both flaky test AND lifecycle gnome evidence. What needs updating is lines 1405-1408:

Current (wrong):
```python
assert task["title"] == "Stabilize run completion guard panic test", task
assert task["files"] == "src/state.rs", task
text = render_task(task, "107", "21:45")
assert "run_completion_guard_reports_error_on_panic" in text, text
```

Replace with (correct):
```python
assert task["title"] == LIFECYCLE_TASK_TITLE, task
# The lifecycle task should NOT be falsy contradicted — its script files are
# no longer checked by _check_code_already_exists (only src/*.rs files).
assert task.get("validated_against_assessment") is not False, (
    f"Lifecycle task should not be falsy contradicted after #144 fix, "
    f"got validated_against_assessment={task.get('validated_against_assessment')}"
)
```

Optionally add an assertion that the rendered text contains lifecycle-related content. Use LIFECYCLE_TASK_TITLE constant (line 14) instead of hardcoded string.

Do NOT change the assessment text at lines 1396-1402 — it's correct test input. Do NOT change any production code — the fixes are already in place. This is purely a test assertion update (~5 lines changed).
