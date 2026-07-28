# Task 01 Obsolete: Fix false contradiction detection in _check_code_already_exists (#144)

**Status:** Obsolete — fix already landed on Day 148.

## Evidence the Fix Is Already In Place

1. **Source code already has the fix.** `_check_code_already_exists()` at line 676-677 of `scripts/preseed_session_plan.py` already filters to only check `src/*.rs` files:
   ```python
   if not (fp.startswith("src/") and fp.endswith(".rs")):
       continue
   ```
   This was committed on Day 148 (commit `2b8d78ac`).

2. **Regression test exists.** Lines 1404-1411 test that the lifecycle task (with script-only files) is NOT falsely contradicted. The assertion `task.get("validated_against_assessment") is not False` passes.

3. **New direct unit test added.** Lines 1413-1424 now add a direct call to `_check_code_already_exists()` with a script-only task (`files: "scripts/preseed_session_plan.py"`, `keys: ("LIFECYCLE_TASK_TITLE",)`) and assert the function returns `(False, "")` — proving script files are skipped even when keywords appear verbatim in them.

4. **All tests pass:**
   ```
   $ python3 scripts/preseed_session_plan.py --test
   preseed_session_plan self-tests passed
   ```

## Why the Issue Was Still Open

The Day 148 task was reverted due to evaluator timeout (no verifier verdict), but a subsequent Day 148 session re-landed the fix in commit `2b8d78ac`. The issue (#144) remained open because the evaluator revert created it, and no session closed it afterward.

## What Was Added

A focused direct-unit-test for `_check_code_already_exists()` that verifies script files are skipped — this makes the #144 fix mechanically verifiable at the function level, not just through the `choose_task` integration test.
