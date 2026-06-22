Title: Reduce reverted-no-edit rate via preseed evidence-aware task selection
Files: scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py
Issue: none
Origin: planner
validated_against_assessment: true

Evidence:
- YOUR TRAJECTORY shows `task states: reverted_no_edit=2` from Day 113 (17:40): two tasks were selected by the manifest but reverted with zero file edits because they had 0 verification evidence in state.
- Graph-derived next-task pressure says "Raise verified task success rate (task_success_rate=0.5)" with reverted_no_edit count as a contributing drag.
- The preseed `choose_task` function matches candidate tasks by keyword against assessment text but does not check whether a candidate's verifier criteria have supporting state evidence. A task with keys that match the assessment but whose success can't be verified produces a wasted implementation slot.
- Day 113 evol flow now honors manifest task selection (evolve.sh skips unselected tasks), so the manifest itself is the remaining gate where unverifiable tasks slip through.

Edit Surface:
- scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If reverted_no_edit is already 0 in the next trajectory or the assessment's recent changes show this was already fixed, write an obsolete-task note. If the preseed self-tests already cover evidence-aware filtering (check the test cases), the task reduces to adding the one missing test case or adjusting the scoring weight.

Objective:
Make preseed task selection evidence-aware: when the trajectory/assessment shows reverted_no_edit pressure, `choose_task` should prefer candidates whose success criteria can be verified with available state evidence, and should demote or annotate candidates that lack verifiable success paths.

Why this matters:
reverted_no_edit=2 means two implementation slots were wasted on tasks that could never produce verifiable success. This directly drags task_success_rate below 0.5 and wastes API budget (~$3-8 per session). The preseed is the first selection point — if it picks landable tasks, the manifest and implementation phases have a real chance.

Success Criteria:
- `choose_task` in preseed_session_plan.py can detect when assessment/trajectory evidence shows reverted_no_edit pressure and adjusts candidate scoring.
- When analysis-only/no-edit pressure is active, candidates that require source-file edits (have non-script Files) are scored higher than analysis-only candidates, since they can produce verifiable commits.
- Self-tests in preseed_session_plan.py cover the evidence-aware path: given an assessment with reverted_no_edit=2, the preseed selects a task whose Files list targets verifiable source changes.
- No protected implementation files appear in the selected task's Files list.

Verification:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_state_graph_tools

Expected Evidence:
- Next trajectory shows reduced reverted_no_edit count (0 or 1 instead of 2).
- Future task manifests show Files entries that correspond to git-tracked source files (not analysis-only scripts with no verifiable output).
- `task_success_rate` metric in log_feedback improves as fewer slots are wasted on unverifiable tasks.

Implementation Notes:
- The seed task was pre-created by the harness. Refine the `choose_task` function and/or the "Make analysis-only task pressure landable" candidate (line 206) in preseed_session_plan.py.
- Key change: when `_has_analysis_only_pressure(metrics)` returns True, the scoring should prefer candidates whose Files list contains source files that can produce git-committable changes (i.e., tasks that touch `src/*.rs` or non-analysis scripts that produce state artifacts), over analysis-only candidates that only modify diagnostic scripts.
- The `_ANALYSIS_ONLY_METRICS` tuple (line 545) already captures the pressure signals. The change is to use this signal to re-rank candidates, not just to skip the lifecycle task.
- If the implementation finds that the preseed already handles this correctly and the gap is downstream in the manifest, narrow the task to adding a preseed self-test that verifies the evidence-aware path and write a note about the manifest gap for the next session.
- Keep the change scoped to the listed files unless verification reveals a direct dependency.
