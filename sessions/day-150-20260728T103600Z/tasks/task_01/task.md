# Task 01: Fix false contradiction detection in _check_code_already_exists (#144)

Title: Fix false contradiction detection in _check_code_already_exists
Files: scripts/preseed_session_plan.py
Issue: #144
Origin: planner (replaces harness-seed lifecycle task — planning pipeline is the #1 bottleneck per assessment)

Evidence:
- Trajectory: planner_no_task_count=1, session_success_rate=0.0, task_seed_contradiction_count=1
- 7 of 9 recent sessions landed zero code changes — the planning pipeline is the dominant bottleneck
- Day 148 assessment identified root cause: `_check_code_already_exists()` runs `git grep` for task keys in task files. When the target file is `scripts/preseed_session_plan.py`, keywords match the script's own TASKS constant definitions and test data — not completed implementation code.
- Day 148 fix (restricting to `src/*.rs`) was correct but reverted due to evaluator timeout, not wrong code
- The assessment confirms: "The preseed task seeder fix (Day 148) removed a self-referential detection bug, but the underlying 'nothing to plan' case still resolves to silence." — wait, the Day 148 fix was actually reverted, so the bug is still live
- `python3 scripts/preseed_session_plan.py --test` still fails at the assertion that checks for false contradictions
- Issue #144 is OPEN with "Task reverted: Fix false contradiction detection in _check_code_already_exists"

Edit Surface:
- scripts/preseed_session_plan.py

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If `_check_code_already_exists` has already been fixed upstream or the --test output shows a different failure (not the false contradiction assertion), mark this task obsolete and explain.
- If restricting to src/*.rs causes the self-test assertion at the fallback path to change, update the test assertion to match the new behavior.

Objective:
Fix `_check_code_already_exists()` so task keywords found in task definition constants, test data, or module-level strings within `scripts/preseed_session_plan.py` itself do not cause false contradiction detection. This unblocks the planning pipeline and ends the empty-session streak.

Why this matters:
The false contradiction detector is the single most likely root cause of the Day 145-150 empty-session streak (7+ sessions with zero code changes). Every task candidate gets scanned for its keywords in its target files; when the target file is `scripts/preseed_session_plan.py`, the keywords match the script's own TASKS constant definitions, test fixtures, and module-level data. The detector declares the task "already done," `choose_task()` finds zero valid candidates, and the planning phase produces no task files — silently killing the session. This directly addresses the trajectory's #1 graph-derived pressure: "Make planning failure actionable (planner_no_task_count=1)."

Success Criteria:
- `python3 scripts/preseed_session_plan.py --test` passes (no AssertionError related to false contradiction)
- Tasks whose `files` include `scripts/preseed_session_plan.py` are no longer falsely contradicted when their keywords appear in the script's own TASKS constants or test data
- Tasks with keywords matching real shipped implementation code in `src/*.rs` files are still correctly detected as already-done
- `planner_no_task_count` drops to zero in the next session

Verification:
- python3 scripts/preseed_session_plan.py --test
- python3 -c "from scripts.preseed_session_plan import _check_code_already_exists; print('import OK')"

Expected Evidence:
- `python3 scripts/preseed_session_plan.py --test` exits 0 with all checks passing
- Next session's planning phase produces at least one valid task file (planner_no_task_count drops)
- The seeded task_01.md for the next session is not falsely contradicted

Implementation Notes:
The simplest fix: restrict `_check_code_already_exists()` to only check `src/*.rs` files, since Rust source code changes are the primary evidence of completed implementation work. Script files contain task definitions and test fixtures that will always match their own keywords.

The function currently iterates over `task.get('files', [])` and runs `git grep` for each key in each file. Add a filter: skip any file that doesn't start with `src/` and end with `.rs`. This is a ~3-line change.

Alternative if that's too restrictive: exclude the file being checked if it's also the file containing the TASKS constant (i.e., `scripts/preseed_session_plan.py`). But the src/*.rs filter is simpler and more correct — the primary evidence of completed implementation work is compiled Rust code that passes tests.

Do NOT change the contradiction detection for `src/*.rs` files — that's working correctly. The bug is specifically that script files contain task definition text that matches their own keywords.

If the self-test at line 1792 (or wherever the assertion lives now) needs updating to match the new behavior, update it. The test should verify that tasks targeting `scripts/preseed_session_plan.py` are NOT falsely contradicted by keywords that only appear in the script's own TASKS constants.
