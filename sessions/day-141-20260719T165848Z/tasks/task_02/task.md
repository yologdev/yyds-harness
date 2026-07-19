Title: Wire semantic title-resolution fallback into contradiction detector
Files: scripts/preseed_session_plan.py
Issue: #124
Origin: planner

Evidence:
- Trajectory (Day 141): `task_obsolete_count=1` — one task was selected that was already completed.
- Issue #124 documents the exact pattern: the unbounded-command task was re-created by the planner even though Task 1 (commit 460a1e03) had already landed 142 lines implementing it in src/safety.rs. The assessment text described completion with informal language that the contradiction detector missed.
- `_line_shows_title_resolution()` (line 549) is defined in preseed_session_plan.py but NEVER called — it's dead code. It was added as a semantic fallback for exactly this case (Day 118 lesson: "shared evidence is not shared understanding when subsystems parse through different dictionaries") but was never wired into `check_task_contradiction()`.
- `check_task_contradiction()` (line 698) has two passes: (1) `_line_shows_resolution` which requires task keys to appear verbatim in the line, (2) `_line_shows_obsolete_or_reverted` which matches obsolete/reverted markers. There is no third pass that uses title-word matching — the gap where `_line_shows_title_resolution` belongs.
- The gap: when assessment prose says "Task 1 already landed this" or "criteria already satisfied by prior work" but the task's `keys` are metric names like `bash_tool_error` that don't appear in those sentences, both existing passes fail and the task gets served as fresh.

Edit Surface:
- scripts/preseed_session_plan.py (wire `_line_shows_title_resolution` into `check_task_contradiction`; add test case)

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If the function is already wired (check the actual source, not memory), mark this task obsolete with evidence.
- If wiring it causes false positives in existing tests, scope the fallback to only trigger when the first two passes both fail.
- Keep the change minimal — one call site addition, one test case.

Objective:
Wire the already-defined `_line_shows_title_resolution()` semantic fallback into `check_task_contradiction()` so that completed-work descriptions using informal language (not matching task keys verbatim) are detected before tasks are served to implementation.

Why this matters:
The Day 118 lesson ("shared evidence is not shared understanding when subsystems parse through different dictionaries") identified this exact failure mode. The fix was written (`_line_shows_title_resolution`) but never activated. This one-line wiring change closes a known gap that caused at least one wasted task slot (issue #124) and directly addresses `task_obsolete_count=1` from the trajectory.

Success Criteria:
- `check_task_contradiction()` calls `_line_shows_title_resolution()` as a third-pass semantic fallback when the first two passes (resolution signals + obsolete markers) both return no contradiction.
- `python3 scripts/preseed_session_plan.py --test` passes, including a new test case where assessment text describes completion using informal language that doesn't contain task keys verbatim.
- The existing `_line_shows_title_resolution` function and its `_RESOLUTION_SIGNALS` tuple are unchanged (already correct).

Verification:
- python3 scripts/preseed_session_plan.py --test
- grep -n '_line_shows_title_resolution' scripts/preseed_session_plan.py — the function should appear at its definition AND at the call site in `check_task_contradiction`.

Expected Evidence:
- Future trajectory: `task_obsolete_count` decreases (fewer already-completed tasks selected for implementation).
- Task manifests show fewer "Task marked obsolete by agent; no implementation landed" outcomes.

Implementation Notes:
- In `check_task_contradiction()` (line 698), after the second pass (line 729-733) and before the self-tests check (line 735), add a third pass:
  ```python
  # Third pass: semantic title-resolution fallback for when task keys
  # don't appear verbatim in resolution prose (e.g. assessment says
  # "Task 1 already landed this" but task keys are metric names).
  for line in recent_changes.splitlines():
      if _line_shows_title_resolution(line, str(task.get("title", ""))):
          return True, f"assessment shows '{task['title']}' problem already resolved (title match): {line.strip()}"
  ```
- Add a test case in the `--test` path that verifies: when assessment text contains "Task 1 already landed the unbounded-command detection" and the task title is "Add unbounded-command warning to bash safety analysis" with keys like `bash_tool_error`, the contradiction detector returns True.
- The function already handles session-date prefixes (line 560: `re.match(r"day\s+\d+", lower)`) and resolution signals (line 562-571). No changes needed to the function itself.
