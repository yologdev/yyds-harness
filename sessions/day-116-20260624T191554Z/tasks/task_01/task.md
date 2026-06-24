Title: Make analysis-only task pressure produce landable implementation tasks
Files: scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py
Issue: none
Origin: planner

Evidence:
- Graph pressure row #1: planner_no_task_count=1 — the planner produced no concrete task files in the most recent session.
- Assessment: 3 of 6 recent sessions produced 0/0 tasks. Sessions assess correctly but the planning pipeline fails to convert assessment into concrete implementation tasks.
- Task-state: reverted_no_edit=2, reverted_unlanded_source_edits=1 — tasks were attempted but produced no landed source changes.
- Log feedback top lesson: "planner produced no usable task → bound discovery and require a selected task artifact before implementation work starts."
- The `_analysis_only_no_terminal_evidence` seed path in preseed_session_plan.py currently selects broad tasks or protected-file harness work when analysis-only/no-edit pressure is active. Those tasks get reverted because they touch protected surfaces or exceed file limits.

Edit Surface:
- scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py

Verifier:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_state_graph_tools

Fallback:
- If the assessment, source, or recent changes show analysis-only/no-edit pressure is already handled correctly for the current evidence, write session_plan/task_01_obsolete.md explaining the exact contradiction and proof.

Objective:
Ensure that when the harness detects analysis-only/no-edit task pressure (planner_no_task_count, reverted_no_edit, task_analysis_only_attempt_count), the preseed script selects a concrete, landable follow-up task with a Files list that contains only non-protected, source-owned files — producing an implementation-ready task_01.md instead of a task that gets reverted.

Why this matters:
This directly addresses the #1 graph pressure row (planner_no_task_count=1). The harness is healthy but idle — build passes, state is clean, cache is efficient — yet sessions frequently produce 0/0 tasks because the planning/task-selection pipeline can't convert analysis-only pressure into landable implementation work. Fixing preseed task selection is the narrowest, highest-leverage change to break the idle cycle without touching protected evolution files.

Success Criteria:
- `python3 scripts/preseed_session_plan.py --test` passes all cases including analysis-only/no-edit pressure paths.
- When `task_analysis_only_attempt_count > 0` or `reverted_no_edit > 0` evidence is present, the preseed script selects a task whose Files list contains no PROTEcted_IMPLEMENTATION_FILES entries.
- The selected task's Files list contains at most 3 source-owned files (scripts/python or src/rust).
- `python3 -m unittest scripts.test_state_graph_tools` passes.

Verification:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_state_graph_tools

Expected Evidence:
- Future task manifests show landable Files entries when analysis-only/no-edit pressure is active.
- planner_no_task_count drops to 0 in subsequent trajectory snapshots.
- reverted_no_edit count decreases because preseed tasks land instead of being reverted.

Implementation Notes:
- This task was seeded by the harness before planner exploration because recent runs reached planning without durable task files.
- The preseed script (`scripts/preseed_session_plan.py`) has an `_analysis_only_no_terminal_evidence` seed path and an `_has_analysis_only_pressure` detection function. The implementation should ensure that when analysis-only pressure is detected, the selected task's Files list is validated against PROTEcted_IMPLEMENTATION_FILES before writing.
- `scripts/state_graph_tools.py` may need a helper to expose protected-file validation for reuse.
- Keep the change scoped to the listed files unless verification reveals a direct dependency.
- Do not modify PROTEcted_IMPLEMENTATION_FILES itself — only add validation that uses the existing list.
