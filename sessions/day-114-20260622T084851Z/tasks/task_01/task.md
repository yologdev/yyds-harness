Title: Make analysis-only task pressure landable
Files: scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py
Issue: none
Origin: harness-seed (refined by planner with Day 114 assessment evidence)
validated_against_assessment: true

Evidence:
- Day 114 04:21 session fixed the word-boundary bug in test-result signals and added src/*.rs preference for reverted_no_edit pressure. The assessment confirms this specific bug is RESOLVED.
- However, the broader analysis-only/no-edit pressure path in preseed still lacks self-tests. When `task_analysis_only_attempt_count` or analysis-only pressure is high, the preseed must select landable, non-protected tasks — and this path has no test coverage to prevent regression.
- The seed was created because recent runs reached planning without durable task files. The word-boundary fix is one step; test coverage is the next.
- Day 112 added analysis-only pressure detection + file-count ceiling to preseed; this code also has no targeted self-tests.

Edit Surface:
- scripts/preseed_session_plan.py, scripts/state_graph_tools.py, scripts/test_state_graph_tools.py

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If preseed self-tests already cover the analysis-only/no-edit pressure path (check with `python3 scripts/preseed_session_plan.py --test` and `python3 -m unittest scripts.test_state_graph_tools`), mark this task obsolete.
- If the analysis-only pressure detection added in Day 112 is no longer in the code, mark obsolete.

Objective:
Add self-test coverage for the analysis-only/no-edit task-pressure path in preseed and state_graph_tools, and verify that protected-file exclusion works correctly when analysis-only pressure is elevated.

Why this matters:
Day 114 04:21 fixed the word-boundary bug that caused reverted_no_edit false triggers, and Day 112 added analysis-only pressure detection with file-count ceilings. But neither change added targeted tests. Without test coverage, the next preseed change could reintroduce the regression (non-landable tasks, protected-file selection). This task closes the test-coverage gap so future preseed changes can be verified automatically.

Success Criteria:
- `python3 scripts/preseed_session_plan.py --test` passes and includes analysis-only pressure test cases
- `python3 -m unittest scripts.test_state_graph_tools` passes
- Preseed self-tests demonstrate that analysis-only/no-edit pressure produces a non-protected, landable Files list
- No protected implementation files appear in selected task Files when analysis-only pressure is active

Verification:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_state_graph_tools
- Manually verify: search test code for "analysis_only" or "no_edit" test cases that validate Files output

Expected Evidence:
- Preseed --test output shows new test cases for analysis-only pressure
- Future task manifests show landable Files entries for task-success repair pressure
- Future trajectory pressure leads with implementation failure repair when analysis-only counts are elevated

Implementation Notes:
- This task was seeded by the harness before planner exploration because recent runs reached planning without durable task files.
- Treat it as a minimum viable task for Day 114 (08:48); refine it if the implementation agent finds stronger evidence, but do not leave the session with zero task files.
- Keep the change scoped to the listed files unless verification reveals a direct dependency.
- Do NOT repeat the word-boundary fix (already done in Day 114 04:21). Focus on test coverage for the analysis-only pressure path.
- If `scripts/preseed_session_plan.py --test` doesn't exist as a flag, add test-mode entry points or test helper functions that the implementation agent can verify manually.
