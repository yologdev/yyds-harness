Title: Fix false contradiction detection in _check_code_already_exists
Files: scripts/preseed_session_plan.py
Issue: none
Origin: planner (replacing contradicted harness-seed)

Evidence:
- `python3 scripts/preseed_session_plan.py --test` FAILS at line 1792: `AssertionError: Expected validated_against_assessment=true, got False`
- The Day 148 assessment identified the root cause: `_check_code_already_exists()` runs `git grep -n -F -- <key> -- <file>` for each task key in each task file. When a task's `keys` include terms like `search_regex_error` and its `files` includes `scripts/preseed_session_plan.py`, the grep finds the key in the script's own TASKS constant definitions and test data — not in completed implementation code.
- The seeded task_01.md was marked `validated_against_assessment: false` with the contradiction message: `code already exists: 'search_regex_error' found in src/tools.rs` — but the grep hit was in a test function name, not in shipped feature code.
- Day 147 had 3 empty sessions with `planner_no_task_count=1` — the planning pipeline produced no task files because `choose_task()` fell through to the "all contradicted" path.
- Trajectory: `planner_no_task_count=1`, `session_success_rate=0.0`

Edit Surface:
- scripts/preseed_session_plan.py

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If `_check_code_already_exists` has already been fixed or the --test output shows a different failure, mark this task obsolete and explain.
- If restricting to src/*.rs causes legitimate completed-script-work to not be detected, that's acceptable — the primary evidence of completed implementation is Rust source changes that pass cargo build && cargo test. Script-only tasks that re-seed are lower cost than the current false-contradiction bug that causes zero-task sessions.

Objective:
Fix `_check_code_already_exists()` so task keywords found in task definition constants, test data, or module-level strings within `scripts/preseed_session_plan.py` itself do not cause false contradiction detection. This unblocks the planning pipeline and ends the empty-session streak.

Why this matters:
The false contradiction detector is the single most likely root cause of the Day 145-147 empty-session streak (4+ sessions with zero code changes). Every task candidate gets scanned for its keywords in its target files; when the target file is `scripts/preseed_session_plan.py`, the keywords match the script's own TASKS constant definitions, test fixtures, and module-level data. The detector declares the task "already done," `choose_task()` finds zero valid candidates, and the planning phase produces no task files — silently killing the session. Fixing this directly addresses the trajectory's #1 graph-derived pressure: "Make planning failure actionable (planner_no_task_count=1)."

Success Criteria:
- `python3 scripts/preseed_session_plan.py --test` passes (no `AssertionError` at line 1792)
- Tasks whose `files` include `scripts/preseed_session_plan.py` are no longer falsely contradicted when their keywords appear in the script's own TASKS constants or test data
- Tasks with keywords matching real shipped implementation code in `src/*.rs` files are still correctly detected as already-done

Verification:
- python3 scripts/preseed_session_plan.py --test

Expected Evidence:
- `python3 scripts/preseed_session_plan.py --test` exits 0 with all checks passing
- Next session's planning phase produces at least one valid task file (planner_no_task_count drops)
- The seeded task_01.md for the next session is not falsely contradicted

Implementation Notes:
The simplest fix: restrict `_check_code_already_exists()` to only check `src/*.rs` files, since Rust source code changes are the primary evidence of completed implementation work. Script files contain task definitions and test fixtures that will always match their own keywords.

The function at line 644 currently iterates over `task.get('files', [])` and runs `git grep` for each key in each file. Add a filter: skip any file that doesn't start with `src/` and end with `.rs`. This is a ~3-line change.

Alternative if that's too restrictive: exclude the file being checked if it's also the file containing the TASKS constant (i.e., `scripts/preseed_session_plan.py`). But the src/*.rs filter is simpler and more correct — the primary evidence of completed implementation work is compiled Rust code that passes tests.

Do NOT change the contradiction detection for `src/*.rs` files — that's working correctly. The bug is specifically that script files contain task definition text that matches their own keywords.
