Title: Add success-rate-aware candidate filtering to preseed task picker
Files: scripts/preseed_session_plan.py
Issue: #121
Origin: planner

Evidence:
- Trajectory graph pressure row #1: `planner_no_task_count=1` — planner produced no concrete task files.
- Trajectory graph pressure row #3: `session_success_rate=0.0` — sessions land no code.
- Assessment: "Multiple sessions landing no code or reverting tasks... the assessment/planning pipeline isn't reliably producing implementable tasks that pass verification."
- Recent window: 1/6 tasks strictly verified. Day 143 (04:13) reverted due to evaluator timeout. Day 142 had `unlanded_source_edits=1`. Day 141 had reverted tasks.
- The `choose_task` function (line 927) already has `analysis_only_active` filtering (line 950-951: rejects >3-file candidates). It does NOT have success-rate-aware filtering — a session with 0.0 success rate gets the same candidates as a session with 1.0.
- Self-issue #121 was reverted due to evaluator timeout (not code failure). The spec was sound but too broad. This is a scoped-down retry: just the `choose_task` sort block, not the full TASKS list reorganization.

Edit Surface:
- scripts/preseed_session_plan.py

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If `task_success_rate` is not present in the assessment metrics (older trajectory format), skip the filter and use existing behavior — no-op.
- If adding the sort block triggers test failures in unrelated tests, scope down further: only add the test case, not the sort block, and let next session decide whether to wire it in.
- If the change touches more than scripts/preseed_session_plan.py, abort and write `session_plan/task_02_obsolete.md`.

Objective:
When `task_success_rate == 0.0` in the assessment metrics, `choose_task` prefers single-file candidates over multi-file candidates. This makes the task picker adaptive to recent outcomes — sessions coming off reverts get smaller, safer tasks.

Why this matters:
The trajectory shows `session_success_rate=0.0` and `task_verification_rate=0.0`. Tasks keep getting reverted because they're too broad for the 20-minute implementation window. When recent sessions show zero success, the task picker should compensate by selecting smaller-scoped candidates. This closes a feedback loop: small wins → higher success rate → unlocks larger tasks.

Success Criteria:
- When `task_success_rate` is 0.0 in metrics, candidates are sorted by file count (fewest files first).
- When `task_success_rate >= 0.5` or absent from metrics, existing behavior is unchanged.
- `python3 scripts/preseed_session_plan.py --test` passes all tests (both existing and new).
- A new test verifies: low-success-rate assessment selects a candidate with fewer files than the default sort.

Verification:
- python3 scripts/preseed_session_plan.py --test

Expected Evidence:
- Future trajectory snapshots show reduced `task_analysis_only_attempt_count` and `reverted_no_edit` when coming off low-success sessions.
- Task manifests show selected tasks with 1-2 file entries when success_rate was 0.0 in prior session.

Implementation Notes:
- In `choose_task` (line 927), the `analysis_only_active` sort block is at lines 970-972:
  ```python
  if analysis_only_active:
      # Prefer tasks touching source files first, then sort by file count
      candidates.sort(key=lambda c: (not any(f.strip().endswith('.rs') for f in c['files'].split(',')), _task_file_count(c)))
  ```
- Add a similar block RIGHT AFTER the `analysis_only_active` block (around line 972), for success-rate awareness:
  ```python
  success_rate = metrics.get("task_success_rate", 1.0)
  if success_rate == 0.0:
      # When recent sessions landed nothing, prefer single-file candidates
      candidates.sort(key=lambda c: _task_file_count(c))
  ```
- `_task_file_count` is at line 891 — splits `files` string by comma and counts.
- `metrics` is already computed at line 930 via `numeric_metrics(lower)`.
- The `success_rate == 0.0` check uses the `numeric_metrics` function which extracts float values from the assessment text. Check how `_has_analysis_only_pressure` (line ~880 area) accesses metrics for the pattern.
- For the test case, in the test function (around line 1390+), add:
  ```python
  def test_choose_task_prefers_single_file_when_success_rate_zero():
      # Assessment with task_success_rate=0.0
      assessment = """
      task_success_rate: 0.0
      task_verification_rate: 0.0
      reverted_no_edit: 2
      planner_no_task_count: 1
      """
      task = choose_task(assessment)
      # Should prefer a candidate with few files
      files = task.get("files", "")
      file_count = len([f for f in files.split(",") if f.strip()])
      assert file_count <= 2, f"Expected <=2 files for low success rate, got {file_count}"
  ```
- Keep the total change under 30 lines. Only touch `choose_task` and the test functions.
- Do NOT modify TASKS list, `_has_analysis_only_pressure`, or any other function.
- Do NOT add new task templates. Just the sort block and one test.
