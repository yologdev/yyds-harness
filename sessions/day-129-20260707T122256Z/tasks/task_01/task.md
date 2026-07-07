Title: Repair evidence-backed planning after no-task sessions
Files: scripts/preseed_session_plan.py, scripts/task_manifest.py, scripts/test_task_manifest.py
Issue: none
Origin: harness-seed
validated_against_assessment: true

Evidence:
- Current assessment matched this harness seed: The harness reached planning with no task artifacts. That makes evolution look healthy while skipping implementation, so planning reliability itself becomes the highest-priority repair.

Edit Surface:
- scripts/preseed_session_plan.py, scripts/task_manifest.py, scripts/test_task_manifest.py

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If current assessment, source, or recent changes show this failure class is already fixed or no longer live, write an obsolete-task note instead of editing.

Objective:
Improve yyds fallback task selection and manifest validation so an evidence-rich assessment is reliably converted into concrete, landable task files.

Why this matters:
The harness reached planning with no task artifacts. That makes evolution look healthy while skipping implementation, so planning reliability itself becomes the highest-priority repair.

Success Criteria:
- Fallback planning repair tasks avoid protected implementation files.
- Task manifest warnings make no-task planning failures visible.
- Future planning failures preserve enough evidence to select a landable repair task.

Verification:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_task_manifest
- python3 scripts/task_manifest.py --help

Expected Evidence:
- Future task manifests show selected task artifacts with non-protected Files entries.
- planning_failed remains visible when it occurs.

Implementation Notes:
- This task was seeded by the harness before planner exploration because recent runs reached planning without durable task files.
- Treat it as a minimum viable task for Day 129 (12:22); refine it if the planner has stronger evidence, but do not leave the session with zero task files.
- Keep the change scoped to the listed files unless verification reveals a direct dependency.
