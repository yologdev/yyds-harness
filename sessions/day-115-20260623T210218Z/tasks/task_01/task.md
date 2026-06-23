Title: Make analysis-only task pressure landable
Files: scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py
Issue: none
Origin: harness-seed
validated_against_assessment: true

Evidence:
- Current assessment matched this harness seed: Recent evo evidence showed implementation attempts ending with no file progress, `reverted_no_edit`, and no terminal evidence. The next seed must target landable task-selection logic so DeepSeek can improve the loop without touching protected evolution files.

Edit Surface:
- scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If current assessment, source, or recent changes show this failure class is already fixed or no longer live, write an obsolete-task note instead of editing.

Objective:
Ensure task-success pressure from analysis-only/no-edit attempts produces a small, landable follow-up task instead of selecting broad or protected-file harness work.

Why this matters:
Recent evo evidence showed implementation attempts ending with no file progress, `reverted_no_edit`, and no terminal evidence. The next seed must target landable task-selection logic so DeepSeek can improve the loop without touching protected evolution files.

Success Criteria:
- Graph-derived analysis-only/no-edit pressure selects a concrete seed before lifecycle cleanup.
- The selected seed Files list contains no protected implementation files.
- Preseed self-tests cover the analysis-only/no-edit pressure path.

Verification:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_state_graph_tools

Expected Evidence:
- Future task manifests show landable Files entries for task-success repair pressure.
- Future trajectory pressure leads with implementation failure repair when `task_analysis_only_attempt_count`, `reverted_no_edit`, or task_success_rate evidence shows no-edit task failure.

Implementation Notes:
- This task was seeded by the harness before planner exploration because recent runs reached planning without durable task files.
- Treat it as a minimum viable task for Day 115 (21:02); refine it if the planner has stronger evidence, but do not leave the session with zero task files.
- Keep the change scoped to the listed files unless verification reveals a direct dependency.
