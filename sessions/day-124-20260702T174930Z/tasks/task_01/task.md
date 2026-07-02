Title: Fix stale fixture task detection — preseed_session_plan.py doesn't check for pre-existing fixture files
Files: scripts/preseed_session_plan.py
Issue: #58
Origin: planner

Evidence:
- Day 124 mid-day (10:41) session was handed "Add held-out coding eval fixture for DeepSeek prompt layout determinism" as Task 2, but fixture 369-deepseek-prompt-layout-determinism.json was already committed on Day 120 (commit 9f3cab05, 4 days prior).
- `git log --follow -- eval/fixtures/local-smoke/369-deepseek-prompt-layout-determinism.json` confirms the fixture exists as valid JSON.
- The preseed task picker's contradiction detector (`check_task_contradiction` in preseed_session_plan.py) checks assessment text for resolution keywords and metric keys, but does NOT check whether files named in a proposed task already exist on disk.
- The assessment text described the task as "the most substantive open issue" with no mention that the fixture file already existed — the contradiction detector had no signal to catch.
- Wasted a full task slot and evaluator time on redundant work.

Edit Surface:
- scripts/preseed_session_plan.py (check_task_contradiction or choose_task, ~lines 200-350)

Verifier:
- python3 scripts/preseed_session_plan.py (runs with --help or --dry-run equivalent)
- Manual check: if a fixture file named in a task already exists on disk, the task should be suppressed

Fallback:
- If the preseed script doesn't have access to the fixture file paths (tasks reference fixture names, not full paths), add a path-resolution step or skip this approach and instead add a git-log check for recently-added fixture files matching the task's target name.
- If the stale contradiction detector already covers this case but missed it due to a bug (not a gap), fix the bug instead of adding a new check.

Objective:
Prevent preseed_session_plan.py from seeding tasks that ask to create fixture files that already exist on disk, by adding a file-existence check to the contradiction detection logic.

Why this matters:
Stale task seeding wastes implementation sessions and erodes trust in the planning pipeline. When a task asks to create a fixture that was already committed days ago, the implementation agent either: (a) creates a duplicate fixture (confusing), (b) discovers the fixture exists and marks the task done without real work, or (c) fails evaluation because the "new file" wasn't actually new. All three are worse than not seeding the task. The trajectory shows task_unlanded_source_edits as the dominant failure mode — some of those may be stale tasks where the work was already done.

Success Criteria:
- When preseed_session_plan.py considers a task that would create/edit a fixture file (matching eval/fixtures/local-smoke/*.json), it checks whether that file already exists on disk via os.path.exists() or Path().exists().
- If the file exists, the task is suppressed (contradiction detected) with a clear log message like "fixture already exists: <path> — suppressing stale task".
- The check must resolve the fixture name to a full path: if a task references "370-deepseek-prompt-layout-determinism-eval", the check should look for eval/fixtures/local-smoke/370-*.json or similar.
- Existing contradiction checks (assessment key scanning, resolution phrase detection) continue to work.
- The script's self-tests pass (python3 scripts/preseed_session_plan.py with inline test mode if available).

Verification:
- python3 -c "import scripts.preseed_session_plan"  (syntax check)
- python3 scripts/preseed_session_plan.py --help  (or equivalent, to confirm it runs)
- Confirm that a task referencing fixture #369 is now suppressed (since 369-deepseek-prompt-layout-determinism.json exists)

Expected Evidence:
- The next session's preseed output no longer proposes creating fixture files that already exist.
- Task manifest shows fewer stale-task contradictions that get through the detector.

Implementation:
1. In `scripts/preseed_session_plan.py`, find the `check_task_contradiction` function (or the place where tasks are validated before selection).
2. Add a file-existence check: for any task whose title or body mentions a fixture file (pattern: `eval/fixtures/local-smoke/NNN-*.json` or bare fixture number like `#369`), resolve the path and check `os.path.exists()`.
3. If the file exists and was committed more than 1 session ago (check git log to avoid false positives from the current session's in-progress work), treat it as a contradiction.
4. Log the suppression clearly so it appears in assessment artifacts.
5. The check should only apply to fixture-creation tasks, not to tasks that modify existing fixtures (which is legitimate work).
