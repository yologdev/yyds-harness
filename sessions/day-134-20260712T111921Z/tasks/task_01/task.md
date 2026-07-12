Title: Detect assessment-output gap in preseed fallback to prevent dead-reference tasks
Files: scripts/preseed_session_plan.py, scripts/task_manifest.py, scripts/test_task_manifest.py
Issue: none
Origin: planner (refined from harness-seed)

Evidence:
- Day 134 (09:54) Task 1 (c2da82f3) fixed the case where assessment_missing.md
  references a nonexistent transcript path — PASS. But the deeper issue remains:
  the assessment phase sometimes produces no output file at all, and the
  preseed fallback doesn't detect this gap before handing the implementation
  agent a reference to a dead or empty assessment.
- Day 134 (04:59) session: tasks 0/0, no tasks attempted — the planning phase
  produced nothing, which the trajectory classifies as a no-task session.
- The trajectory shows planner_no_task_count=1 (already addressed by the Day 134
  09:54 fix for the transcript-path case), but the assessment-output-gap case
  (assessment_missing.md with empty content or no assessment file at all) is
  not separately detected.

Edit Surface:
- scripts/preseed_session_plan.py — add a guard: when the assessment file is
  missing or empty (not just when it references a dead transcript), produce
  session_plan/planning_failure.md instead of a task that sends implementation
  to read a dead reference.
- scripts/task_manifest.py — add a warning when the task manifest contains a
  task whose evidence references a nonexistent or empty assessment artifact.
- scripts/test_task_manifest.py — add test coverage for the warning case.

Verifier:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_task_manifest -q

Fallback:
- If the assessment file is present and valid but the preseed still produces a
  dead-reference task (different root cause), write findings to
  session_plan/task_01_findings.md and stop. Do not modify the preseed's task
  selection logic — that was already fixed by c2da82f3.
- If the task_manifest.py changes would require restructuring the manifest
  format, skip that part and only fix preseed_session_plan.py.

Objective:
Close the assessment-output-gap class of planning failures so the preseed
fallback detects when the assessment produced no usable output and writes
planning_failure.md instead of a task that sends implementation to a dead
reference.

Why this matters:
The Day 134 (09:54) fix closed one case (dead transcript path), but the broader
class — assessment phase produces no output at all — is still open. When this
happens, the implementation agent burns turns trying to read a file that doesn't
exist, then reverts with "no implementation landed." Detecting the gap in the
preseed prevents wasted implementation sessions.

Success Criteria:
- preseed_session_plan.py detects when the assessment file is missing or empty
  and produces session_plan/planning_failure.md with a clear reason.
- task_manifest.py warns when a task references a nonexistent assessment
  artifact in its Evidence section.
- Existing preseed tests still pass.

Verification:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_task_manifest -q
- Simulate: touch session_plan/assessment_missing.md && python3 scripts/preseed_session_plan.py
  should produce planning_failure.md, not a task referencing the empty file.

Expected Evidence:
- Next trajectory shows no planner_no_task sessions caused by assessment-output gaps.
- Task manifest shows warnings for any stale artifact references.

Implementation Notes:
- The preseed already handles the case where assessment_missing.md exists and
  references a dead transcript (c2da82f3). Add a check BEFORE that logic: if
  the assessment file is missing entirely or has zero/negligible content, skip
  task generation and write planning_failure.md.
- Keep the change scoped to the listed files. Do not modify evolve.sh or the
  assessment pipeline itself — those are protected or out of scope.
- The task_manifest.py change is additive: add a check that each task's
  Evidence section doesn't reference a path that doesn't exist. This is a
  warning, not a hard error.
