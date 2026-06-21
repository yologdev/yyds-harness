Title: Make analysis-only task pressure landable
Files: scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py
Issue: none
Origin: harness-seed (refined by planner)
validated_against_assessment: true

Evidence:
- Day 111 session: 1/2 tasks, with one reverted_no_edit (no file progress, no terminal evidence).
- Day 113 session 2: 0/1 tasks, obsolete_already_satisfied (preseed picked a task for already-implemented work).
- Trajectory graph pressure still carries reverted_no_edit and task_analysis_only_attempt_count signals.
- Current 10-session win streak means this is preventative — the pressure exists but isn't acute.
- Assessment confirms these failure modes are live in the preseed logic even when sessions succeed.

Edit Surface:
- scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py

Verifier:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_state_graph_tools

Fallback:
- If the implementation agent finds that the analysis-only/no-edit pressure path is already adequately handled (e.g., the Day 113 11:17 fix resolved the stale-detection issue and the obsolete_already_satisfied case is one-off), write a task_01_obsolete.md explaining the evidence and stop. Do not edit code if the problem is already fixed.

Objective:
When graph-derived pressure shows analysis-only/no-edit task attempts (reverted_no_edit, task_analysis_only_attempt_count, low task_success_rate), the preseed script must select a concrete, small, landable follow-up task instead of a broad or protected-file harness task.

Why this matters:
The preseed script is the harness's first line of task selection. When it picks tasks that are too broad, touch protected files, or are already satisfied, the implementation agent wastes a session. The Day 113 11:17 1-line fix (word-boundary match for "fail"/"error") was a step forward; this task continues hardening the selection logic for the analysis-only pressure path.

Success Criteria:
- Graph-derived analysis-only/no-edit pressure selects a concrete, landable seed (Files list contains no protected implementation files, task is completable in 20 min).
- Preseed self-tests cover at least one analysis-only/no-edit pressure scenario.
- Existing tests continue to pass.

Verification:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_state_graph_tools

Expected Evidence:
- Future task manifests show landable Files entries when task-success repair pressure is present.
- Future trajectory pressure leads with implementation failure repair when reverted_no_edit or task_success_rate evidence shows no-edit task failure.
- Preseed self-test output includes coverage for the analysis-only pressure path.

Implementation Notes:
- This task was seeded by the harness before planner exploration.
- The implementation agent should focus on the `choose_task` and `TASKS` dict in preseed_session_plan.py — the logic that maps evidence pressure to a specific task.
- Protected implementation files are listed in PROTECTED_IMPLEMENTATION_FILES in preseed_session_plan.py — the agent must ensure selected tasks don't include these.
- If the analysis-only pressure path already selects a small, landable task in current code, verify with --test and write a confirmation note instead of editing.
- Day 113 11:17 already fixed word-boundary matching for "fail"/"error" detection; do not re-fix that.
