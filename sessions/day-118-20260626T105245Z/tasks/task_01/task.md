Title: Fix stale/obsolete seed detection in preseed contradiction check  
Files: scripts/preseed_session_plan.py  
Issue: #39  
Origin: planner  

Evidence:  
- Day 118 dawn session (03:50Z): Task 1 "Make analysis-only task pressure landable" was marked OBSOLETE by the implementation agent because all three success criteria were already mechanically satisfied in the codebase. The assessment line read: `Day 118 (03:50) | 3 tasks | 2/3 verified. Task 1 (analysis-only pressure) marked obsolete — criteria already satisfied.`  
- The harness re-seeded the same task for this session (10:52Z) because `check_task_contradiction` (line 505) didn't catch it: none of the task keys match "obsolete" or "criteria already satisfied" as substrings, and `_analysis_only_seed_recently_blocked` (line 630) doesn't list "obsolete" or "already satisfied" in its blocked_markers.  
- `task_obsolete_count=1` appears in the trajectory task-state summary but is not checked by any staleness detection function.  
- Command: `python3 scripts/preseed_session_plan.py --test` passes (all existing tests green), confirming the code is mechanically correct — the gap is in staleness detection, not in the preseed logic itself.  

Edit Surface:  
- `scripts/preseed_session_plan.py` — `check_task_contradiction` (line 505), `_analysis_only_seed_recently_blocked` (line 630), and preseed self-tests (lines ~1439+)  

Verifier:  
- `python3 scripts/preseed_session_plan.py --test`  

Fallback:  
- If the assessment already contains a "stale seed detection now catches obsolete" line (meaning a prior session already landed this fix), mark this task obsolete with the exact line as evidence. Do not re-implement.  

Objective:  
Make `check_task_contradiction` detect when a task's problem domain was recently marked `obsolete_already_satisfied` or `reverted_no_edit` in the current assessment's task-state evidence, so stale seeds don't reach implementation.  

Why this matters:  
The graph pressure row "Replace stale or already-satisfied tasks (task_obsolete_count=1)" directly names this failure. A stale seed costs one full task slot (~$1.50-3 in API costs) and blocks landing real work. This is the #1 actionable bug from the Day 118 assessment — the staleness detection has survived multiple fix attempts without catching this case.  

Success Criteria:  
- `check_task_contradiction` returns `(True, reason)` when the assessment's Recent Changes section contains evidence that a task with matching keys was marked obsolete or reverted without edits.  
- `_analysis_only_seed_recently_blocked` recognizes "obsolete" and "criteria already satisfied" as blocking markers.  
- The preseed self-tests include a regression test where a task whose keys match an assessment line containing "marked obsolete — criteria already satisfied" is correctly flagged as contradicted.  

Verification:  
- `python3 scripts/preseed_session_plan.py --test`  
- Manual: feed the assessment text from Day 118 dawn session into the preseed and confirm the analysis-only task is NOT selected.  

Expected Evidence:  
- Preseed self-test output shows a new passing test for obsolete-seed contradiction detection.  
- After this fix, future sessions with `obsolete_already_satisfied=1` in task-state evidence will not re-select the same stale seed.  
- The `task_obsolete_count` metric in future trajectory output trends toward zero.  

Implementation Notes:  

### Root Cause

The `check_task_contradiction` function (line 505) scans the assessment's Recent Changes section for lines that match task keys AND contain resolution signals. It has two detection paths:

1. **Day-prefix path**: `re.match(r"day\s+\d+", lower)` — catches lines starting with "Day NNN" that contain a task key.  
2. **Resolution-signal path**: `_RESOLUTION_SIGNALS` tuple — catches phrases like "now properly", "already fixed", "has been resolved", etc.

The Day 118 assessment line `Day 118 (03:50) | 3 tasks | 2/3 verified. Task 1 (analysis-only pressure) marked obsolete — criteria already satisfied.` matched the Day-prefix path BUT none of the analysis-only task's keys are substrings of this line. The keys are: `"task_analysis_only_attempt_count"`, `"reverted_no_edit"`, `"no-edit revert"`, `"implementation ended without file progress"`, etc. None of these appear in "analysis-only pressure marked obsolete — criteria already satisfied."

### Fix (Three Changes)

**Change 1**: In `check_task_contradiction` (line 505), after the existing `_line_shows_resolution` check, add a second pass that detects task-state evidence patterns:

```python
# After existing check at line ~517, add:
for line in recent_changes.splitlines():
    if _line_shows_obsolete_or_reverted(line, task_keys):
        return True, f"assessment shows '{task['title']}' problem domain already obsolete/reverted: {line.strip()}"
```

The new helper `_line_shows_obsolete_or_reverted(line, task_keys)` should return True when:
- The line contains any task key (substring match)
- AND the line contains one of: `"marked obsolete"`, `"obsolete_already_satisfied"`, `"reverted_no_edit"`, `"reverted — no edit"`, `"criteria already satisfied"`, `"reverted without"`

**Change 2**: In `_analysis_only_seed_recently_blocked` (line 630), add to `blocked_markers`:
```python
"obsolete",
"criteria already satisfied",
"marked obsolete",
```

**Change 3**: Add a regression test in the preseed self-tests (around line 1333 where stale-seed tests already exist). Create an assessment text snippet that includes `"Day 118 (03:50) | 3 tasks | 2/3 verified. Task 1 (analysis-only pressure) marked obsolete — criteria already satisfied."` and assert that the analysis-only task (with its real keys) is flagged as contradicted.

### Scope Note

This task only touches `scripts/preseed_session_plan.py`. The related `scripts/task_manifest.py` contradiction check (which also missed this case per the assessment) is a separate, lower-priority concern — the preseed is the first and most impactful gate. Fixing preseed staleness detection prevents stale seeds from being written to task files in the first place.
