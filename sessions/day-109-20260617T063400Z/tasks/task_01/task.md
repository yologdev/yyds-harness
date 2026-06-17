Title: Repair evidence-backed planning after no-task sessions
Files: skills/evolve/SKILL.md, skills/self-assess/SKILL.md, scripts/task_manifest.py
Issue: none
Origin: harness-seed
validated_against_assessment: true

Evidence:
- Current assessment matched this harness seed: The harness reached planning with no task artifacts. That makes evolution look healthy while skipping implementation, so planning reliability itself becomes the highest-priority repair.

Edit Surface:
- skills/evolve/SKILL.md, skills/self-assess/SKILL.md, scripts/task_manifest.py

Verifier:
- python3 -m unittest scripts.test_task_manifest

Fallback:
- If current assessment, source, or recent changes show this failure class is already fixed or no longer live, write an obsolete-task note instead of editing.

Objective:
Improve yyds planning guidance and task manifest validation so an evidence-rich assessment is reliably converted into concrete task files.

Why this matters:
The harness reached planning with no task artifacts. That makes evolution look healthy while skipping implementation, so planning reliability itself becomes the highest-priority repair.

Success Criteria:
- The planning skill explicitly prioritizes writing task artifacts before extra exploration.
- Task manifest warnings make no-task planning failures visible.
- Future planning failures preserve enough evidence to select a repair task.

Verification:
- python3 -m unittest scripts.test_task_manifest
- python3 scripts/task_manifest.py --help

Expected Evidence:
- Future dashboard sessions show selected task artifacts instead of an empty implementation phase.
- planning_failed remains visible when it occurs.

Implementation Notes:
- This task was seeded by the harness before planner exploration because recent runs reached planning without durable task files.
- Treat it as a minimum viable task for Day 109 (06:34); refine it if the planner has stronger evidence, but do not leave the session with zero task files.
- Keep the change scoped to the listed files unless verification reveals a direct dependency.
