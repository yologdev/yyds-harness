Title: Add success-rate-aware candidate sorting to preseed task picker
Files: scripts/preseed_session_plan.py
Issue: #121 (scoped down)
Origin: planner

Evidence:
- Trajectory: `task_success_rate=0.0`, `task_verification_rate=0.0`,
  `task_analysis_only_attempt_count=2`, `task_unlanded_source_count=1`.
  4 of 5 recent sessions (Days 139-142) had all tasks reverted or zero tasks
  landed. Issue #121 attempted this fix but the evaluator timed out — this
  version is scoped to a single sort block only.
- Graph pressure row 1: "Raise verified task success rate (task_success_rate=0.0):
  Dominant task failure: task_unlanded_source_count=1 (source edits not landed).
  Consider smaller, self-contained tasks."
- The `choose_task` function in preseed_session_plan.py already has
  `analysis_only_active` filtering and `_task_file_count` helper. It treats
  all sessions the same regardless of recent success rate.
- Issue #126 (today): "all 1 selected tasks reverted" — the task picker
  selected a task that was too large for the session.

Edit Surface:
- scripts/preseed_session_plan.py: add one sort block in `choose_task`
  (after the existing `analysis_only_active` sort, around line 970-972)
  that sorts candidates by `_task_file_count` when `task_success_rate`
  is 0.0 or `reverted_no_edit` > 0. Add one test case in the `--test` path.

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If `task_success_rate` is absent from assessment metrics, skip the sort
  and use existing behavior (backward compatible).
- If the sort removes all candidates (should not happen — sort doesn't
  filter), fall through to existing fallback logic unchanged.
- If adding the test case requires restructuring the test harness, skip
  the test and verify manually instead. Do not refactor the test harness
  for this change.

Objective:
When recent sessions have a task success rate of 0.0 (or reverted_no_edit > 0),
the preseed task picker prefers candidates touching fewer files, increasing
the probability of selecting a task that can be completed and verified within
the session budget.

Why this matters:
The trajectory consistently shows all tasks reverted when success rate is low.
The task picker currently treats all sessions the same regardless of recent
outcomes. A session coming off a 0/2 revert should get smaller, safer tasks
than a session coming off 2/2 success. This creates a positive feedback loop:
small wins build confidence, which unlocks larger tasks.

Success Criteria:
- When `task_success_rate` is 0.0 or `reverted_no_edit` > 0 in the
  assessment metrics passed to `choose_task`, candidates are sorted by
  file count (fewest files first).
- When `task_success_rate` >= 0.5 and `reverted_no_edit` == 0, existing
  behavior is unchanged.
- When `task_success_rate` is absent from metrics, existing behavior is
  unchanged (backward compatible).
- `python3 scripts/preseed_session_plan.py --test` passes.

Verification:
- python3 scripts/preseed_session_plan.py --test

Expected Evidence:
- Future trajectory snapshots show `task_success_rate` > 0.0 within 2-3
  sessions after this change lands.
- Task manifests show selected tasks with 1-2 Files entries when
  success_rate was 0.0 in the prior session.

Implementation Notes:
- In `choose_task` (near line 970-972, after the `analysis_only_active`
  sort block), add a similar sort block:
  ```python
  # When recent sessions have zero task success rate, prefer
  # single-file candidates that are easier to complete and verify.
  success_rate = metrics.get("task_success_rate", 1.0)
  reverted_no_edit = metrics.get("reverted_no_edit", 0)
  if success_rate == 0.0 or reverted_no_edit > 0:
      candidates.sort(key=lambda c: _task_file_count(c))
  ```
- `_task_file_count` already exists in the module (used at line ~950).
  It splits the `files` string by comma and counts entries.
- This is a sort, not a filter — all candidates remain available.
  Candidates with the same file count preserve their relative order.
- Test case: construct a mock assessment with `task_success_rate=0.0`
  and mock candidates with varying file counts. Verify the selected
  candidate has the lowest file count among eligible candidates.
- Keep the change minimal — one sort block (5 lines), one test case
  (~30 lines). Do not modify the TASKS list or add new templates.
  Do not touch any other function.
