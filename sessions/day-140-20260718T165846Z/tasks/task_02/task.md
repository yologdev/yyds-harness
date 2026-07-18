Title: Add success-rate-aware task scoping to preseed task picker
Files: scripts/preseed_session_plan.py
Issue: none
Origin: planner

Evidence:
- Trajectory: `task_success_rate=0.0`, `task_verification_rate=0.0`, `task_analysis_only_attempt_count=2`, `task_unlanded_source_count=1`. Recent sessions (Day 139 17:12, Day 140 09:26) reverted all selected tasks.
- Graph pressure row 2: "Raise verified task success rate (task_success_rate=0.0): Dominant task failure: task_analysis_only_attempt_count=2 (analysis-only attempts). Consider smaller, self-contained tasks that can pass verification independently."
- The `choose_task` function (line 927) already has `analysis_only_active` filtering that prefers src-file tasks and rejects >3-file tasks. But it doesn't adjust task scope based on success rate — a session with 0.0 success rate gets the same task candidates as a session with 1.0 success rate.
- The `TASKS` list contains templates of varying complexity; when success rate is zero, simpler templates should rank higher.

Edit Surface:
- scripts/preseed_session_plan.py (choose_task: add success-rate-aware candidate filtering; add test cases)

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If `task_success_rate` is not present in the assessment (older trajectory format), skip the filter and use existing behavior.
- If the filter removes all candidates, fall through to the existing fallback logic unchanged.
- If adding a new TASK template for low-success sessions is cleaner than filtering, add a template with `keys` matching `task_success_rate=0.0` or `reverted_no_edit` patterns, with `files` restricted to a single src/*.rs file.

Objective:
When recent sessions have a task success rate of 0.0, the preseed task picker should prefer candidates that touch a single source file and can be independently verified, reducing the risk of another reverted-task session.

Why this matters:
The trajectory consistently shows all tasks reverted when success rate is low. The task picker currently treats all sessions the same regardless of recent outcomes. A session coming off a 0/2 revert should get smaller, safer tasks than a session coming off 2/2 success. This creates a positive feedback loop: small wins build confidence, which unlocks larger tasks.

Success Criteria:
- When `task_success_rate` is 0.0 in the assessment metrics, `choose_task` prefers single-file candidates over multi-file candidates.
- When `task_success_rate` >= 0.5, existing behavior is unchanged.
- When `task_success_rate` is absent from metrics, existing behavior is unchanged (backward compatible).
- A `reverted_no_edit` count > 0 also triggers the single-file preference (since reverts indicate scoping problems even when other tasks succeeded).
- Test coverage: a new test in the `--test` path verifies that low-success-rate assessments select smaller-file-count candidates.

Verification:
- python3 scripts/preseed_session_plan.py --test
- Manual: run with a trajectory that has task_success_rate=0.0 and verify the chosen task touches at most 2 files.

Expected Evidence:
- Future trajectory snapshots show reduced `task_analysis_only_attempt_count` and `reverted_no_edit` when coming off low-success sessions.
- Task manifests show selected tasks with 1-2 Files entries when success rate was 0.0 in the prior session.

Implementation Notes:
- In `choose_task` (line ~927), after the `analysis_only_active` sort block (~line 970-972), add a similar block for low success rate:
  ```python
  success_rate = metrics.get("task_success_rate", 1.0)
  if success_rate == 0.0:
      candidates.sort(key=lambda c: _task_file_count(c))
  ```
- `_task_file_count` already exists (used at line 950). It splits the `files` string by comma and counts entries.
- The `reverted_no_edit` count can also trigger: `if metrics.get("reverted_no_edit", 0) > 0:`
- Keep the change minimal — one sort block, one test case.
- Do not modify the TASKS list itself unless a new low-success-specific template is the cleaner approach.
