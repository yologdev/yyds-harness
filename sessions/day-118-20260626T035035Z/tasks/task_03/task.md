Title: Add eval fixture for empty-session reason classification
Files: eval/fixtures/local-smoke/empty-session-reason-classification.json
Issue: #37
Origin: planner

Evidence:
- Issue #37: "Add held-out coding eval coverage for DeepSeek harness gnomes" — specifically requests eval fixtures for harness diagnostic gaps
- Assessment Day 118: 60% no-op rate without root-cause classification, empty-streak tracker added Day 117 but no eval fixture validates it
- YOUR TRAJECTORY: fitness_score=0.667, diagnostic gnomes lack held-out eval baselines
- Existing eval fixture pattern: eval/fixtures/local-smoke/*.json files with task_id, category, goal, tests, hidden_failure_mode, expected_files, risk_label

Edit Surface:
- eval/fixtures/local-smoke/empty-session-reason-classification.json (new file)

Verifier:
- python3 -c "import json; json.load(open('eval/fixtures/local-smoke/empty-session-reason-classification.json')); print('valid JSON')"

Fallback:
- If task_02 (empty-session reason classification) is obsolete or reverted, mark this fixture obsolete too since it validates the same feature. Write session_plan/task_03_obsolete.md.

Objective:
Add a held-out eval fixture that validates empty-session reason classification behaves correctly across the three reason classes (assessment_empty, implementation_failed, reverted_no_edit), providing a test baseline separate from unit tests and task verification.

Why this matters:
The capability fitness gnome `task_success_rate` is measured at 0.667 with no held-out eval baseline — we don't know if the measurement itself is accurate. Adding an eval fixture for the empty-session diagnostic gives the harness a stable, version-controlled truth to compare against across sessions. This is specifically the kind of "held-out coding eval evidence" Issue #37 requests: a fixture that can validate harness diagnostic behavior independently of the task verification loop.

Success Criteria:
- A new JSON fixture file exists at eval/fixtures/local-smoke/empty-session-reason-classification.json
- The fixture follows the existing format: task_id, category, goal, tests, hidden_failure_mode, expected_files, risk_label
- The fixture references the empty-session classification functions from extract_trajectory.py (added in task_02)
- JSON is valid and parseable

Verification:
- python3 -c "import json; json.load(open('eval/fixtures/local-smoke/empty-session-reason-classification.json')); print('valid JSON')"
- python3 -m unittest scripts.test_extract_trajectory  (verifies the functions the fixture references)

Expected Evidence:
- The fixture appears in `ls eval/fixtures/local-smoke/` as a new file
- Future `yyds evaluate` or dashboard runs can reference this fixture
- The fixture documents the expected behavior contract for empty-session classification

Implementation Notes:
- This task is purely additive: one new file, no changes to existing code.
- Use the format from existing fixtures like eval/fixtures/local-smoke/036-deepseek-transport-error-policy.json as a template:
  ```json
  {
    "task_id": "empty-session-reason-classification",
    "category": "deepseek/harness self-diagnosis",
    "repo_fixture": "self",
    "initial_commit": "current",
    "goal": "Validate that empty-session classification correctly distinguishes assessment_empty from implementation_failed from reverted_no_edit using per-session outcome/artifact evidence.",
    "tests": [
      "python3 -m unittest scripts.test_extract_trajectory.ExtractTrajectoryTests.test_classify_empty_session_reason_assessment_empty",
      "python3 -m unittest scripts.test_extract_trajectory.ExtractTrajectoryTests.test_classify_empty_session_reason_implementation_failed",
      "python3 -m unittest scripts.test_extract_trajectory.ExtractTrajectoryTests.test_classify_empty_session_reason_reverted_no_edit"
    ],
    "hidden_failure_mode": "The classifier conflates assessment_empty with reverted_no_edit, causing the harness to apply the wrong intervention (e.g., trying to force edits when the real problem is assessment finding nothing to work on).",
    "expected_files": [
      "scripts/extract_trajectory.py",
      "scripts/test_extract_trajectory.py"
    ],
    "risk_label": "low"
  }
  ```
- The test function names in `tests` should match whatever task_02 creates. If task_02 uses different test names, adjust accordingly.
- If task_02 hasn't landed yet (this session), the fixture should still be written with forward-looking test names. The fixture validates the design contract; it's OK if the tests don't exist yet — that's the point of held-out eval.
- risk_label should be "low" — this is a diagnostic fixture, not a protocol or safety test.
