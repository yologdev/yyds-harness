Title: Harden preseed task picker with git-tracked file check
Files: scripts/preseed_session_plan.py
Issue: none
Origin: planner

Evidence:
- Trajectory shows 7 reverted_no_edit tasks across 5 recent sessions (Day 110-111 window). The root cause: the preseed picker selects tasks pointing at files that don't exist or are not part of the working tree.
- Day 111 added a file-existence check to preseed_session_plan.py, but this only catches non-existent files — it doesn't catch gitignored files, generated artifacts, or files outside the repo that happen to exist on disk.
- The assessment confirms: "Preseed task picker still occasionally selects tasks pointing at renamed/moved files (addressed Day 111 with file-existence check, but may need further hardening)"

Edit Surface:
- scripts/preseed_session_plan.py

Verifier:
- python3 scripts/preseed_session_plan.py (self-test or --help)
- python3 -c "import py_compile; py_compile.compile('scripts/preseed_session_plan.py', doraise=True)"

Fallback:
- If the file-existence check from Day 111 already covers all real cases and the 7 reverted_no_edit tasks were caused by a different mechanism (e.g., assessment/planning timing), write a findings note and mark the task as no-repro. Do not add git-tracked checks that duplicate existing protection.

Objective:
Reduce reverted_no_edit tasks to zero by ensuring the preseed picker only selects tasks whose target files are git-tracked and present in the working tree.

Why this matters:
Reverted_no_edit tasks waste a full task slot: the implementation agent starts, discovers the file doesn't exist or isn't editable, and reverts without making progress. Seven of these across 5 sessions means ~7 wasted 20-minute implementation windows. Each wasted slot is a missed opportunity to improve the harness.

Success Criteria:
- The preseed picker skips tasks whose Files: entries point to untracked, gitignored, or non-existent paths.
- The picker still selects tasks when all Files: entries are git-tracked and present.
- The existing PROTECTED_IMPLEMENTATION_FILES check continues to work correctly.

Verification:
- python3 scripts/preseed_session_plan.py (basic syntax/runtime check)
- python3 -c "import py_compile; py_compile.compile('scripts/preseed_session_plan.py', doraise=True)"
- Manual: run preseed in a scenario where a task targets a gitignored file and confirm it's skipped

Expected Evidence:
- Future sessions: reverted_no_edit task count drops to zero (or near-zero, excluding non-file-related reverts).
- State events: fewer TaskReverted events with "file not found" or "not in working tree" reasons.
- Dashboard: task_success_rate improves as fewer slots are wasted on unreachable files.

Implementation Notes:
- Add a helper that checks `git ls-files --error-unmatch <path>` (or equivalent) for each file in a task candidate's Files: line.
- Integrate this check into the existing task selection logic — likely in `choose_task()` or the file-existence check added Day 111.
- The check should be cheap (one git command or cached git ls-files output), not a full repo scan per candidate.
- If git is not available (unlikely in this repo), fall back to the existing file-existence check rather than blocking task selection entirely.
- Do not modify the TASKS dictionary or lifecycle task definitions — only the selection/filtering logic.
