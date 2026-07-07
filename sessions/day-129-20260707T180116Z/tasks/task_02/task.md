Title: Exclude file-less tasks from implementation selection in task manifest
Files: scripts/task_manifest.py
Issue: #79
Origin: planner

Evidence:
- Issue #79: Day 129 (13:07) session reverted because task file had no Files: entries. Verification gate returned `planned_files: []`.
- task_manifest.py line 299-300 already detects `missing_files` and emits a warning, but the `selectable` filter at lines 310-323 does NOT exclude tasks with empty `files`.
- Since `missing_files` tasks are not filtered from `selectable`, they can be picked for implementation, then fail verification with "task file has no Files: entries."
- The trajectory shows task_success_rate=0.0 from the most recent session with `scope_mismatch=1` as the dominant failure mode.

Edit Surface:
- scripts/task_manifest.py

Verifier:
- python3 -m unittest scripts.test_task_manifest

Fallback:
- If `selectable` already filters out tasks without files (check the actual code), mark this task as already-resolved with an evidence note.

Objective:
Prevent tasks with empty `Files:` entries from being selected for implementation, so they fail fast at the planning/manifest stage instead of wasting an implementation session that ends in scope-mismatch reversion.

Why this matters:
Task manifest warnings that don't block selection are invisible to the downstream pipeline. A warning at line 300 (`missing_files`) that doesn't prevent selection at line 310 means the task still gets sent to implementation, where it inevitably fails verification. Making the filter reject file-less tasks moves the failure from implementation-time (expensive — burns a session) to planning-time (cheap — the planner can pick a different task or fix the file).

Success Criteria:
- Tasks with `not task.get("files")` are excluded from `selectable`.
- When all tasks are excluded due to missing files, `no_selectable_tasks` warning fires (existing behavior at line 326).
- Existing tests in `test_task_manifest.py` continue to pass.

Verification:
- python3 -m unittest scripts.test_task_manifest

Expected Evidence:
- Future task manifests never select file-less tasks for implementation.
- `no_selectable_tasks` warning fires when all candidates have missing files, instead of silently selecting a doomed task.
- Issue #79 resolution: tasks that would have been reverted by the verification gate are caught at the manifest stage instead.

## Implementation

In `task_manifest.py`, add `not task.get("files")` to the `selectable` list comprehension filter (lines 310-323). The current filter is:

```python
selectable = [
    task
    for task in tasks
    if not task.get("protected_files")
    and not (
        isinstance(task.get("quality"), dict)
        and task["quality"].get("analysis_only_escape")
    )
    and not (
        isinstance(task.get("quality"), dict)
        and isinstance(task["quality"].get("assessment_alignment"), dict)
        and task["quality"]["assessment_alignment"].get("contradicted_by_assessment")
    )
]
```

Add a condition:

```python
    and task.get("files")  # reject file-less tasks — they always fail verification
```

This is defense-in-depth: Task 01 already ensures the preseed script always populates files, but this manifest-level guard catches any future regression.

If the tests in `test_task_manifest.py` don't cover the file-less task case, add one test that verifies a task with empty files is excluded from `selectable`.
