Title: Add success-rate-aware candidate sorting to preseed task picker
Files: scripts/preseed_session_plan.py
Issue: #121 (scoped down — single sort block only)
Origin: planner

Evidence:
- Graph-derived pressure #1: "Raise verified task success rate (task_success_rate=0.5): Dominant task failure: task_unlanded_source_count=1 (source edits not landed). Consider smaller, self-contained tasks that can pass verification independently." (trajectory 2026-07-19T02:51Z).
- Day 140 (16:58): 1/2 strict verified, task states include `reverted_unlanded_source_edits=1`. Day 140 (10:39): 0/2 strict verified, `reverted_no_edit=1, reverted_unlanded_source_edits=1` (assessment).
- The preseed task picker currently treats all sessions the same regardless of recent success rate. A session coming off 0/2 or 1/2 reverts gets the same task candidates as a session coming off 2/2 success.
- The previous attempt (#121) was reverted due to evaluator timeout (not a code defect). The task scope is reduced: only the sort block in `choose_task`, no TASKS list changes, no new template.
- `choose_task` at line 927 already has an `analysis_only_active` sort block (lines 970-972) that prefers src-file candidates. The new sort block follows the same pattern for low success rate.
- `_task_file_count` already exists at line 891. `numeric_metrics` extracts `task_success_rate` from assessment text.

Edit Surface:
- scripts/preseed_session_plan.py (add success-rate-aware sort block in `choose_task`, add one test case)

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If `task_success_rate` is not present in the metrics (older trajectory format), skip the sort — existing behavior unchanged.
- If the sort removes all candidates (unlikely since it only reorders), fall through to the existing candidate loop unchanged.
- If adding a test case is complex due to metric extraction from assessment text, skip the test and rely on the existing `--test` suite passing.

Objective:
When recent sessions have a low task success rate, the preseed task picker should prefer candidates that touch fewer files, reducing the risk of another reverted-task session.

Why this matters:
The trajectory consistently shows low-success sessions followed by more reverts. The task picker currently treats all sessions identically. A session coming off 0.5 success rate should get smaller, safer tasks than a session coming off 1.0 success. This creates a positive feedback loop: small wins build confidence, which unlocks larger tasks. This directly addresses the self-referential problem noted in the assessment: "the fix for reverted tasks keeps getting reverted" — the preseed picker itself needs to be success-rate-aware to break the cycle.

Success Criteria:
- When `task_success_rate` is <= 0.5 in the metrics, `choose_task` prefers candidates with fewer files over candidates with more files (stable sort, secondary to existing sort keys).
- When `task_success_rate` > 0.5 or absent from metrics, existing behavior is unchanged.
- A `reverted_no_edit` count > 0 also triggers the file-count preference.
- `python3 scripts/preseed_session_plan.py --test` passes.
- Existing test cases are not broken.

Verification:
- python3 scripts/preseed_session_plan.py --test
- Manual: run with a trajectory that has task_success_rate=0.5 and verify the chosen task touches at most 2 files.

Expected Evidence:
- Future trajectory snapshots show reduced `reverted_no_edit` and `reverted_unlanded_source_edits` when coming off low-success sessions.
- Task manifests show selected tasks with 1-2 Files entries when success rate was <= 0.5 in the prior session.

Implementation Notes:
- In `choose_task` (line ~927), after the existing `analysis_only_active` sort block (lines 970-972), add:
  ```python
  # When recent task success rate is low, prefer smaller candidates (fewer files)
  # to reduce reverted-task risk and build a positive feedback loop.
  success_rate = metrics.get("task_success_rate", 1.0)
  reverted = metrics.get("reverted_no_edit", 0)
  if success_rate <= 0.5 or reverted > 0:
      candidates.sort(key=lambda c: _task_file_count(c))
  ```
- `_task_file_count` (line 891) splits the `files` string by comma and counts entries.
- Use `<= 0.5` threshold (not `== 0.0`) because 0.5 (1/2) also indicates fragility.
- The sort is stable (Python's `list.sort` is stable), so within same file-count groups, existing ordering is preserved.
- Keep the change minimal — one sort block, zero changes to TASKS list.
- Add ONE test case in the `--test` path: supply assessment text with `task_success_rate=0.5` and verify the chosen task has fewer files than when `task_success_rate=1.0`.
