Title: Break preseed fallback self-reference when no src/ work exists
Files: scripts/preseed_session_plan.py
Issue: none
Origin: planner (refined from harness-seed)
validated_against_assessment: true

Evidence:
- Trajectory graph pressure: planner_no_task_count=1, task_artifact_coverage=0, task_success_rate=0.0
- The last 4 sessions (Day 114 afternoon through Day 115) touched only scripts and journals — no src/ changes
- The preseed fallback task at scripts/preseed_session_plan.py:703-732 creates a self-referential "Repair evidence-backed planning" task that modifies the same preseed script, producing a cycle where strict verification never passes (no cargo build && cargo test)
- Day 114 commits already addressed two of the original three success criteria: protected-file avoidance (_has_protected_files at line 563), manifest warnings for planning_failed (task_manifest.py line 264-265, commit 895a9cd)
- The remaining gap: when assessment honestly finds nothing to fix in src/, the fallback should not fabricate a self-referential pipeline-fix task

Edit Surface:
- scripts/preseed_session_plan.py

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If the fallback task at line 703 no longer produces self-referential output, mark this task obsolete_already_satisfied.

Objective:
Make the preseed fallback task produce an honest "no src/ work needed" artifact when assessment evidence shows a healthy codebase, instead of a self-referential pipeline-fix task that cycles without ever passing strict verification.

Why this matters:
When the codebase is healthy and assessment finds nothing to fix, the pipeline should record that honestly rather than fabricating a task that modifies planning scripts. The current self-referential fallback creates sessions that look like they did work (modifying scripts) but never achieve strict verification (no cargo build && cargo test), producing a trajectory pattern of 0/1 verified sessions that erodes trust in the harness.

Success Criteria:
- When choose_task() reaches the fallback because all candidates are contradicted AND assessment evidence shows no src/ bugs, the fallback task Files line names only non-code artifacts (e.g., journals/) or explicitly states "no source changes needed"
- The fallback task still fires for genuine pipeline bugs (contradiction from stale evidence, not from healthy codebase)

Verification:
- python3 scripts/preseed_session_plan.py --test

Expected Evidence:
- Future sessions with genuinely healthy codebases produce task files that don't self-modify the planning pipeline
- Strict verification doesn't fail because the task was designed for a no-src-change session

Implementation Notes:
- Two of the original three success criteria from the harness seed are already satisfied by Day 114 commits (10f832d, 895a9cd): protected-file avoidance and manifest warnings for planning_failed
- Focus ONLY on the remaining gap: the fallback at line 703-732 should detect when the contradiction reason is "no src/ work exists" vs "pipeline bug" and produce appropriate output
- The assessment.md artifact show "the code is stable, the harness is healthy" — this is a valid outcome, not a failure to plan
- Keep the change scoped to scripts/preseed_session_plan.py unless verification reveals a direct dependency on task_manifest.py
