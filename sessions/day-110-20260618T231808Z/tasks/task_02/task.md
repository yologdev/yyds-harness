Title: Add pre-dispatch file-existence gate for task Files
Files: scripts/evolve.sh
Issue: none
Origin: planner

Evidence:
- Trajectory: 3 recent sessions with `reverted_no_edit` tasks (Day 110 19:43, 18:44, 12:16)
- Assessment: "implementation agents accepting tasks they don't execute" is the dominant failure mode
- Graph pressure: "Force reverted tasks to leave concrete evidence (task_no_edit_revert_count=1)"
- Current harness: no pre-dispatch validation that task Files entries actually exist in the repo
- The protected-file check (evolve.sh lines 1899-1961) validates file paths are not protected, but does NOT validate they exist

Edit Surface:
- scripts/evolve.sh

Verifier:
- bash -n scripts/evolve.sh  (syntax check)
- grep -c 'file-existence gate\|file.*exists\|file_paths_missing\|all_files_missing' scripts/evolve.sh

Fallback:
- If evolve.sh already has a file-existence check between the protected-file gate and task dispatch, write session_plan/task_02_obsolete.md with the exact line numbers proving it.

Objective:
Before dispatching a task to the implementation agent, validate that at least one file listed in the task's Files/Edit Surface actually exists in the repo. If none exist, write an obsolete note and skip the task — preventing the implementation agent from burning a DeepSeek call on a task with no valid edit surface.

Why this matters:
Three recent sessions had `reverted_no_edit` tasks where implementation agents accepted tasks but produced no file changes. Some of these may have been caused by task files referencing non-existent source paths — a problem the harness can catch before spending an expensive implementation attempt. This gate complements the preseed improvement (task_01) by catching Files-path drift between planning and implementation.

Success Criteria:
- When a task file lists Files that have zero existing entries in the repo, the harness skips the task with an obsolete note and does NOT launch the implementation agent
- When at least one listed file exists, the task proceeds to implementation normally
- The check does not false-positive on tasks whose Files entries are scripts/ or non-src/ paths that do exist
- The obsolete note includes the task title and the list of missing files

Verification:
- bash -n scripts/evolve.sh
- grep -A5 'file-existence' scripts/evolve.sh | head -20

Expected Evidence:
- Next session with a task that references non-existent files should show "Task N skipped — no Files entries exist in repo" (or similar) in the harness output
- `reverted_no_edit` count should trend down in future trajectory snapshots as tasks with wrong file paths are caught before dispatch

Implementation Notes:
- Insert the check between the protected-file gate (which ends around line 1961 with `fi`) and the pre-task SHA capture (line 1964: `if ! PRE_TASK_SHA=...`)
- Extract Files and Edit Surface entries the same way the protected-file check does (inline Python reading the task file)
- For each file path, strip whitespace/quotes/backticks, then test with `[ -f "$file" ]` in bash
- If all extracted files are missing, write `session_plan/${TASK_ID}_obsolete.md` with the task title, the full list of missing files, and a note that the harness blocked dispatch
- Record the skip as a task outcome event (similar to lines 1892-1895 for the contradiction check)
- Increment TASK_FAILURES and continue to the next task
- This check runs AFTER the protected-file check (so protected files are caught first with their own message) and BEFORE the implementation agent launch
- The check should be forgiving: if the Files line lists `src/foo.rs, src/bar.rs` and only `src/foo.rs` exists, the task proceeds (at least one valid target exists)
- Only skip when zero listed files exist
