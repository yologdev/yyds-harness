Title: Make analysis-only task pressure landable
Files: scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py
Issue: none
Origin: harness-seed
validated_against_assessment: true

Evidence:
- Trajectory: 3 recent sessions with `reverted_no_edit` tasks (Day 110 19:43, 18:44, 12:16)
- Assessment: "implementation agents accepting tasks they don't execute" is the dominant failure mode
- Graph pressure: "Force reverted tasks to leave concrete evidence (task_no_edit_revert_count=1)"
- Current preseed `choose_task()` filters for protected files (line 519) but does NOT validate that candidate task files actually exist in the repo
- The analysis-only task template (TASKS list, lines 188-234) currently selects `scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py` — these exist now but could drift

Edit Surface:
- scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If current assessment, source, or recent changes show this failure class is already fixed or no longer live, write an obsolete-task note instead of editing.

Objective:
Add file-existence validation to the preseed's `choose_task()` so that when analysis-only/no-edit pressure is detected, the selected candidate task's Files entries are verified to exist in the repo. If the first candidate has missing files, skip it and try the next candidate. This prevents the preseed from generating task_01.md with non-existent file paths.

Why this matters:
The dominant failure mode in recent sessions is implementation agents reverting without edits. Part of this may be caused by task files referencing non-existent source paths. The preseed should not generate tasks that are guaranteed to fail because their target files don't exist. This complements the evolve.sh pre-dispatch gate (task_02) by catching file-path drift at task generation time rather than at dispatch time.

Success Criteria:
- `choose_task()` validates that at least one file in each candidate's `files` field exists in the repo
- Candidates where zero files exist are skipped; the next candidate is tried
- The fallback task (lines 549-574) also has its files validated
- Preseed self-tests cover: (a) candidate with all-existing files selected, (b) candidate with some-missing files still selected, (c) candidate with all-missing files skipped
- The selected seed Files list contains no protected implementation files (existing behavior preserved)

Verification:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_state_graph_tools

Expected Evidence:
- Future task manifests from preseed show Files entries that all resolve to existing repo paths
- `reverted_no_edit` count trends down as tasks with phantom file paths stop reaching implementation
- Preseed test output shows the new file-existence validation paths

Implementation Notes:
- This task was seeded by the harness before planner exploration because recent runs reached planning without durable task files.
- Treat it as a minimum viable task for Day 110 (23:18); refine it if the planner has stronger evidence, but do not leave the session with zero task files.
- Keep the change scoped to the listed files unless verification reveals a direct dependency.
- Add a helper like `_candidate_files_exist(candidate)` that checks `os.path.isfile()` for each file in the candidate's `files` field (split on comma, strip whitespace). Return True if at least one file exists.
- Integrate the check into the candidate loop in `choose_task()` (around line 514-534), after the `_has_protected_files()` check (line 519) and before appending to candidates.
- Also apply the check to the fallback task (line 549-574) by validating its files field before returning.
- When a candidate fails the file-existence check, `continue` to the next candidate rather than erroring out.
- Update the self-test in `check_task_contradiction` tests or add a new test case in the `--test` path that verifies file-existence filtering works.

