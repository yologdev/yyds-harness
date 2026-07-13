Title: Add cross-reference mismatch detection to task manifest quality scoring
Files: scripts/task_manifest.py scripts/test_task_manifest.py
Issue: none
Origin: planner (from trajectory: task_unlanded_source_count=1, task_verification_rate=0.0)

Evidence:
- Day 135 trajectory: task_unlanded_source_count=1 — a task touched source files without landing a source commit. Likely cause: strict verifier rejected the task due to scope mismatch between Files line and actual edits.
- Day 135 trajectory: task_verification_rate=0.0 — tasks are failing strict verification. One failure mode is the task body mentioning files/symbols not declared in the Files line, causing the verifier to flag edits as out of scope.
- Concrete example: the harness-seeded task_01.md had Files: scripts/preseed_session_plan.py but its Objective said "Analyze the harness dispatching logic in evolve.sh" — a cross-reference mismatch that would cause verifier rejection.
- Task manifest already extracts declared_files from Files line and file_mentions from body text (FILE_MENTION_RE). It computes protected_surface_match. But it does not check whether files mentioned in the body are absent from the Files line.

Edit Surface:
- scripts/task_manifest.py (add cross-reference quality check)
- scripts/test_task_manifest.py (add test cases)

Verifier:
- python3 -m unittest scripts.test_task_manifest

Fallback:
- If task_manifest.py already detects cross-reference mismatches, mark this task obsolete with line references.
- If the test suite path is wrong (scripts.test_task_manifest vs test_task_manifest.py), use whatever import path the existing tests use.

Objective:
Add a quality check to task_manifest.py that detects when a task body mentions file paths (via FILE_MENTION_RE) that are NOT present in the task's Files line. Lower the quality score for tasks with cross-reference mismatches, and surface the mismatched paths in the quality dict so the dashboard/trajectory can flag them.

Why this matters:
Tasks with Files-vs-body mismatches are structurally broken: the implementation agent may edit files not declared in the Files line, and the strict verifier will reject those edits as out-of-scope. Catching this at planning time prevents wasted implementation sessions. This directly addresses task_verification_rate=0.0 and task_unlanded_source_count=1.

Success Criteria:
- task_manifest.py detects when a task body mentions file paths not in the Files line.
- The quality dict includes a new field (e.g., "cross_reference_mismatch" or "undeclared_file_mentions") listing the mismatched paths.
- Quality score is lowered for tasks with cross-reference mismatches.
- Existing manifest tests still pass; new test covers the mismatch case.

Verification:
- python3 -m unittest scripts.test_task_manifest
- python3 scripts/task_manifest.py --session-plan-dir session_plan (manual smoke test)

Expected Evidence:
- Next session: task_manifest quality scores flag cross-reference mismatches.
- Dashboard: task_quality_score reflects structural issues; fewer tasks reach implementation with Files-vs-body mismatches.
- task_verification_rate improves (fewer verifier rejections due to scope mismatch).

Implementation Notes:
- FILE_MENTION_RE already extracts file paths from the body. Compare these against declared_files (parsed from the Files: line).
- Exclude files that are mentioned but clearly read-only (e.g., in "Check X before editing" context). Start simple: any file path in the body not in Files: is a mismatch.
- The quality score adjustment should be moderate — don't zero out the score for one mismatch, but make it visible.
- protected_surface_match already does something similar for protected files — follow that pattern.
- Add a test fixture task with a cross-reference mismatch to test_task_manifest.py.
