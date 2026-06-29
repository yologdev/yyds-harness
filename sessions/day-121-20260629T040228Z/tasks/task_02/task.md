Title: Break analysis-only → analysis-task selection loop in preseed picker
Files: scripts/preseed_session_plan.py
Issue: #45
Origin: planner (narrowed from reverted Day 118 and Day 120 attempts)

Evidence:
- Trajectory Day 120 graph-derived pressure: "Make planning failure actionable (planner_no_task_count=1): The planner produced no concrete task files." and "Make source-edit outcomes land or explain reverts (task_unlanded_source_count=1)"
- Assessment: "Five of the last six sessions produced zero code changes." The preseed picker continues to select the ANALYSIS_ONLY_TASK_TITLE ("Make analysis-only task pressure landable") when analysis-only pressure is the top signal — creating a self-reinforcing cycle where being stuck generates more tasks about being stuck.
- Issue #45 (auto-filed Day 120): Previous narrower attempt at this fix was reverted because "Evaluator timed out without a verifier verdict" — not because the code change failed.
- Current code at line 743 skips ANALYSIS_ONLY_TASK_TITLE only when `_analysis_only_seed_recently_blocked(lower)` is true (the task was already blocked in a previous session). It does NOT skip when `analysis_only_active` is true — meaning the picker will select the analysis-only task even when analysis-only pressure is the dominant signal.
- `_has_analysis_only_pressure(metrics)` already exists (line 681-683) and correctly detects when `task_analysis_only_attempt_count > 0`, `task_no_edit_revert_count > 0`, or `reverted_no_edit > 0` are present.
- `analysis_only_active` is already computed at line 731 and used for candidate sorting at line 769 — the variable exists and is correct. The only gap is that line 743 doesn't consult it.

Edit Surface:
- scripts/preseed_session_plan.py: change the condition at line 743 from `_analysis_only_seed_recently_blocked(lower)` to `(_analysis_only_seed_recently_blocked(lower) or analysis_only_active)`. Add one test case that constructs evidence with `task_analysis_only_attempt_count=1` and asserts the picker returns a landable task, not ANALYSIS_ONLY_TASK_TITLE.

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If the condition at line 743 already includes `analysis_only_active` (check with `grep -n 'analysis_only_active' scripts/preseed_session_plan.py`), write an obsolete_already_satisfied note.
- If the existing test at line 1145-1152 already covers this case (analysis_only_active=True → skips ANALYSIS_ONLY_TASK_TITLE), write obsolete_already_satisfied.

Objective:
When trajectory evidence shows analysis-only/no-edit task pressure (`task_analysis_only_attempt_count > 0`, `task_no_edit_revert_count > 0`, or `reverted_no_edit` in recent sessions), the preseed picker must not select ANALYSIS_ONLY_TASK_TITLE ("Make analysis-only task pressure landable"). Instead it should select the next landable task in the TASKS list. This breaks the self-reinforcing cycle where analysis pressure selects analysis tasks.

Why this matters:
The preseed pipeline is the harness's last chance to turn diagnostic evidence into a concrete task before the planner runs. When analysis-only pressure selects yet another analysis task, the session is doomed before implementation even starts. This single condition change converts one guaranteed-empty session into a session with at least a chance of landing code. Five of the last six sessions landed zero code — this is the root cause.

Success Criteria:
- When `task_analysis_only_attempt_count > 0` appears in evidence text, `choose_task()` does not return a task whose title equals `ANALYSIS_ONLY_TASK_TITLE`.
- A landable alternative task is selected instead (one whose Files list contains at least one editable source file).
- Self-tests cover this escape-hatch path with a new test case.
- Existing tests (including the test at line 1145 that verifies recently-blocked seeds are skipped) continue to pass.

Verification:
- python3 scripts/preseed_session_plan.py --test
- grep -n 'analysis_only_active' scripts/preseed_session_plan.py  (confirm the variable is used in the skip condition)

Expected Evidence:
- Future task manifests for sessions with analysis-only pressure show landable tasks (not "Make analysis-only task pressure landable").
- Future trajectory shows fewer `reverted_no_edit` and `task_analysis_only_attempt_count` outcomes.
- Session day-121 (or whichever session lands this) shows at least one task that landed code.

Implementation Notes:
- The fix is ONE LINE: change line 743 from:
  ```python
  if task["title"] == ANALYSIS_ONLY_TASK_TITLE and _analysis_only_seed_recently_blocked(lower):
  ```
  to:
  ```python
  if task["title"] == ANALYSIS_ONLY_TASK_TITLE and (_analysis_only_seed_recently_blocked(lower) or analysis_only_active):
  ```
- `analysis_only_active` is already computed at line 731 and used at line 769 for candidate sorting. No new variables needed.
- Add ONE test case (after existing tests, before `if __name__ == "__main__"`) that constructs assessment text containing `task_analysis_only_attempt_count=1` and asserts `choose_task(assessment)["title"] != ANALYSIS_ONLY_TASK_TITLE`.
- The test should also assert the returned task has src/*.rs files (or at minimum, doesn't equal ANALYSIS_ONLY_TASK_TITLE).
- Do NOT refactor `choose_task` broadly. Do NOT change the TASKS list, constants, or evidence pipeline. The change is literally one condition.
- After editing, run `python3 scripts/preseed_session_plan.py --test` to confirm all tests pass.
