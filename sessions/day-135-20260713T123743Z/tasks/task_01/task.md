Title: Add cross-reference mismatch detection to task manifest quality scoring
Files: scripts/task_manifest.py, scripts/test_task_manifest.py
Issue: #103
Origin: planner (refined from harness-seed + #103 revert evidence)

Evidence:
- Day 135 trajectory: task_verification_rate=0.333 — only 1/3 tasks passed strict verification. Dominant failure: task_unlanded_source_count=1 (source edits not landed because verifier rejected them as out-of-scope).
- Day 135 (11:12) session: task_03 reverted with reason "task changes do not overlap planned Files entries" — the task body mentioned files to edit but those files were absent from the Files: line.
- Concrete example: the harness-seeded task_01.md had Files: scripts/preseed_session_plan.py but its Objective said "Analyze the harness dispatching logic in evolve.sh" — a cross-reference mismatch that would cause verifier rejection if implemented.
- task_manifest.py already extracts declared_files (from Files: line, line 298) and body file mentions (via FILE_MENTION_RE + extract_file_mentions, lines 22-134, 299). It merges them into `files` (line 299). But it does NOT detect when body mentions are absent from declared_files — the exact gap that causes verifier scope-mismatch rejections.
- The quality dict (lines 312-321) has no cross_reference_mismatch field. Warnings (lines 344-359) don't flag mismatches.

Edit Surface:
- scripts/task_manifest.py: add cross-reference mismatch detection in parse_task(), add to quality dict and warnings
- scripts/test_task_manifest.py: add test fixture task with Files-vs-body mismatch, verify it's detected

Verifier:
- python3 -m unittest scripts.test_task_manifest

Fallback:
- If task_manifest.py already detects cross-reference mismatches (body file mentions absent from Files line), mark this task obsolete with exact line references.
- If the test suite import path is wrong, use the path shown by existing test imports.

Objective:
Catch Files-vs-body cross-reference mismatches at planning time so the strict verifier doesn't reject implementation work as out-of-scope. This directly raises task_verification_rate by preventing a structural failure mode.

Why this matters:
The graph-derived next-task pressure says "Raise verified task success rate (outcome_task_success_rate=0.333)" and "Make source-edit outcomes land or explain reverts (task_unlanded_source_count=1)." The dominant failure mode is tasks getting reverted because the implementation agent edited files not declared in the Files: line. Catching the mismatch at planning time (when the task manifest scores tasks) prevents wasted implementation sessions.

Cross-reference mismatches are structural task defects: the planner wrote about files it didn't declare, so the implementation agent correctly edits those files, and the strict verifier correctly rejects them as out-of-scope. Every party is doing the right thing with the information they have — the fix is to surface the mismatch earlier so the planner can either add the file to Files: or remove the reference from the body.

Success Criteria:
- task_manifest.py detects when task body mentions file paths (via FILE_MENTION_RE) that are NOT present in the task's Files: line.
- The quality dict includes a new field `cross_reference_mismatch` listing the undeclared paths (empty list if none).
- Quality score is lowered for tasks with mismatches (e.g., cap at 0.8 or subtract 0.1 per mismatch, minimum 0.3).
- A warning `task_NN:cross_reference_mismatch` is emitted when mismatches are found.
- Existing manifest tests still pass.

Verification:
- python3 -m unittest scripts.test_task_manifest
- python3 scripts/task_manifest.py --session-plan-dir session_plan --assessment-file session_plan/assessment.md 2>&1 | head -40

Expected Evidence:
- Next session: task manifest quality scores flag cross-reference mismatches in the dashboard.
- task_verification_rate improves (fewer verifier rejections due to scope mismatch).
- task_unlanded_source_count decreases (source edits land because Files: line matches body).

Implementation Notes:
- `extract_file_mentions()` (line 133) returns file paths from the body. Compare these against `declared_files` (line 298, the Files: line entries).
- `declared_files` is computed at line 298, `files` at line 299. Add the mismatch check between these two lines — compute `undeclared = [f for f in body_mentions if f not in declared_files]`.
- Exclude files that are mentioned in read-only context. Start simple: if a file is in the body but not in Files:, flag it. We can add read-only heuristics later.
- The quality score adjustment should be moderate. Current score is sum of 6 booleans / 6.0. Cap at 0.8 or subtract 0.1 per mismatch (min 0.3).
- Follow the pattern of `assessment_alignment` (line 289-294): compute the mismatch, add to quality dict, conditionally lower score.
- Add to warnings following the pattern at lines 349-359.
- Test fixture: create a task text with Files: "src/main.rs" but body mentioning "src/lib.rs" — verify mismatch is detected and quality score is lowered.
- `protected_surface_match` (line 137) already does similar path checking — follow that pattern for the mismatch detection.
- Do NOT modify scripts/preseed_session_plan.py. The preseed task's protected-file avoidance concern is valid but a separate issue; keep this task focused on the cross-reference gap which has direct trajectory evidence.
