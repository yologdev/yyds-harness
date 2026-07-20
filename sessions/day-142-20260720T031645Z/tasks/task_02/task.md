Title: Add success-rate-aware task scoping to preseed task picker
Files: scripts/preseed_session_plan.py
Issue: #121
Origin: planner

Evidence:
- Trajectory: `task_success_rate=0.0`, `task_verification_rate=0.0`, `reverted_unlanded_source_edits` dominant across Day 140-141 sessions.
- Day 141 18:49: 0/2 strict verified, task states: reverted_unlanded_source_edits=2.
- Day 141 04:17: 1/2 strict verified, task states: reverted_unlanded_source_edits=1.
- Day 140 18:14: 1/2 strict verified, task states: reverted_unlanded_source_edits=1.
- Day 140 10:39: 0/2 strict verified, task states: reverted_no_edit=1, reverted_unlanded_source_edits=1.
- Graph pressure row: "Raise session success rate (session_success_rate=0.0)."
- Issue #121 was previously reverted because the evaluator timed out, not because the code was wrong. The issue body contains a complete, minimal implementation plan.
- `choose_task` (line ~927) already has `analysis_only_active` filtering and `_task_file_count`. Adding a success-rate-aware sort block is ~10 lines.

Edit Surface:
- scripts/preseed_session_plan.py (one sort block in `choose_task`, one test case)

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If `task_success_rate` is not present in the assessment metrics, skip the filter entirely (existing behavior).
- If the filter removes all candidates, fall through to existing fallback logic unchanged.
- If the sort block causes any test failure, revert the block and diagnose — the `--test` path must stay green.
- If `_task_file_count` is not importable in the context where the sort runs, use inline logic instead.

Objective:
When recent sessions have a task success rate of 0.0 or a `reverted_no_edit` count > 0, the preseed task picker should prefer candidates that touch fewer source files, reducing the risk of another reverted-task session.

Why this matters:
The trajectory consistently shows `reverted_unlanded_source_edits` across Day 140-141 sessions — tasks where code was written but didn't pass the verifier. This suggests tasks are too ambitious for the available implementation time. The task picker currently treats all sessions the same regardless of recent outcomes. A session coming off a 0/2 revert should get smaller, safer tasks than a session coming off 2/2 success. This creates a positive feedback loop: small wins build confidence, which unlocks larger tasks.

Success Criteria:
- When `task_success_rate` is 0.0 in the assessment metrics, `choose_task` prefers fewer-file candidates over multi-file candidates.
- When `task_success_rate` >= 0.5, existing behavior is unchanged.
- When `task_success_rate` is absent from metrics, existing behavior is unchanged (backward compatible).
- A `reverted_no_edit` count > 0 also triggers the fewer-file preference (since reverts indicate scoping problems even when other tasks succeeded).
- Test coverage: a new test in the `--test` path verifies that low-success-rate assessments select smaller-file-count candidates.
- `python3 scripts/preseed_session_plan.py --test` passes with no regressions.

Verification:
- python3 scripts/preseed_session_plan.py --test
- Manual: run with a trajectory that has task_success_rate=0.0 and verify the chosen task touches at most 2 files.

Expected Evidence:
- Future trajectory snapshots show reduced `reverted_unlanded_source_edits` and `reverted_no_edit` when coming off low-success sessions.
- Task manifests show selected tasks with 1-2 Files entries when success rate was 0.0 in the prior session.

Implementation Notes:
- In `choose_task` (around line 927), after the `analysis_only_active` sort block (~line 970-972), add a similar block for low success rate:
  ```python
  success_rate = metrics.get("task_success_rate", 1.0)
  reverted_no_edit = metrics.get("reverted_no_edit", 0)
  if success_rate == 0.0 or reverted_no_edit > 0:
      candidates.sort(key=lambda c: _task_file_count(c))
  ```
- `_task_file_count` already exists (used at line 950). It splits the `files` string by comma and counts entries.
- Keep the change minimal — one sort block, one test case. Do not modify the TASKS list.
- The test should: create a mock metrics dict with `task_success_rate=0.0`, create two candidate tasks with different file counts, verify the smaller-file-count candidate sorts first.
- If `_task_file_count` raises on a None or missing `files` key, handle gracefully (return a large number to sort to end).
