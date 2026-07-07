Title: Guarantee non-empty Files entries in every preseed task file
Files: scripts/preseed_session_plan.py
Issue: #79
Origin: planner

Evidence:
- Issue #79: Day 129 (13:07) session reverted because task file had no Files: entries. Verification gate returned {"ok": false, "reason": "task file has no Files: entries", "planned_files": []}.
- Task was the fallback "Repair evidence-backed planning after no-task sessions" (line 847-889 of preseed_session_plan.py).
- The render_task function (line 993-1004) writes `Files: {task["files"]}` — this should always produce a non-empty line, but the task["files"] value can become empty through fallback paths that reset or clear it.
- Lines 873-878 have two paths that overwrite `fallback["files"]` to a single file, which is safe, but the broader issue is that no post-render validation exists to catch an empty Files: line before writing to disk.

Edit Surface:
- scripts/preseed_session_plan.py

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If `_candidate_files_exist` already guards every path and the existing tests at lines 1119, 1133, 1151 already assert non-empty files, mark this task verified with an evidence note and skip code edits.

Objective:
Ensure every task file written by preseed_session_plan.py has a non-empty `Files:` line, preventing the scope-mismatch reversion that blocks code from shipping.

Why this matters:
Issue #79 and #80 show that tasks without `Files:` entries are reverted by the verification gate. This is the single highest-impact gap — it directly blocks code from shipping. The trajectory shows task_success_rate=0.0 from the most recent session, with the dominant failure being scope mismatch on tasks with empty planned_files. The assessment identifies this as the #1 capability gap.

Success Criteria:
- render_task raises or returns a diagnostic when task["files"] is empty or whitespace-only.
- All existing fallback paths (lines 847-889, 955-990) populate `files` with at least one existing repo path before calling render_task.
- `python3 scripts/preseed_session_plan.py --test` passes all existing tests, including the assertions at lines 1119, 1133, 1151 that check for non-empty files.

Verification:
- python3 scripts/preseed_session_plan.py --test
- grep -c 'Files:' on generated task output must be >= 1 for every path through the script.

Expected Evidence:
- Future task files written by the preseed script always contain `Files:` with at least one repo path.
- The verification gate no longer rejects tasks with "task file has no Files: entries."
- Issue #79 and #80 become closable.

## Implementation

Add a guard in `render_task` (line 993) that validates `task["files"]` is non-empty before rendering. If empty, raise a ValueError with a diagnostic message naming the task title and the fallback path that produced it.

The guard should be:

```python
def render_task(task: dict[str, object], day: str, session_time: str) -> str:
    files_val = str(task.get("files") or "").strip()
    if not files_val:
        raise ValueError(
            f"Task '{task.get('title', 'unknown')}' has empty files — "
            f"the planning pipeline will reject this task during verification. "
            f"Fix the fallback path that produced this empty-files task."
        )
    # ... existing render logic
```

Also audit every fallback path that sets `task["files"]`:
- Line 849: fallback has 3 files ✓
- Line 875: fallback["files"] = "scripts/preseed_session_plan.py" ✓  
- Line 878: fallback["files"] = "scripts/preseed_session_plan.py" ✓
- Line 964: _healthy_codebase_fallback has "journals/JOURNAL.md" ✓
- Line 989: task["files"] = "journals/JOURNAL.md" ✓

All existing paths already set non-empty files. The guard is the safety net for future changes.

If the existing tests at lines 1119, 1133, 1151 already assert non-empty files and all paths are covered, the implementation agent may mark this task as verified with a test run without adding a new guard — the test suite is the guard.
