Title: Harden preseed fallback task selection and manifest validation
Files: scripts/preseed_session_plan.py, scripts/task_manifest.py, scripts/test_task_manifest.py
Issue: none
Origin: harness-seed (refined by planner)

Evidence:
- Harness seed task was auto-generated because older sessions (pre-Day 131) had no-task planning runs.
- Current trajectory contradicts the crisis premise: Day 132 (12:05) 1/1 strict verified, Day 131 (12:18) 2/2 strict verified.
- However, Day 132 (04:02) and Day 131 (18:37) were no-task sessions — these were clean-tree sessions, not planning failures, but the preseed script can't distinguish the two.
- The defensive improvements (protected-file avoidance in fallback tasks, manifest visibility for no-task sessions) are still worth landing to prevent a real planning failure from being invisible.

Edit Surface:
- scripts/preseed_session_plan.py, scripts/task_manifest.py, scripts/test_task_manifest.py

Verifier:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_task_manifest

Fallback:
- If the preseed test suite reveals these fixes are already applied (e.g., Day 131's preseed improvements already covered the same ground), mark this task obsolete.
- If the manifest test reveals the task would produce no net change (all protections already in place), write an obsolete note.

Objective:
Ensure that when preseed_session_plan.py generates fallback tasks, they avoid protected implementation files (evolve.sh, GitHub workflows, IDENTITY.md, etc.) and that task_manifest.py surfaces no-task planning failures visibly so future sessions can distinguish "clean tree, nothing to do" from "planning broke and nobody noticed."

Why this matters:
A preseed fallback task that suggests editing protected files gets reverted by the verification gate — wasting a session. And a planning failure that produces zero task files looks identical to a clean-tree session in the trajectory. Making both cases more visible prevents silent failures.

Success Criteria:
- Preseed fallback tasks avoid referencing protected implementation files (`PROTECTED_IMPLEMENTATION_FILES` or equivalent list)
- Task manifest shows a warning when no task files exist after planning
- Existing preseed and manifest tests continue to pass
- No changes to src/ Rust code — Python scripts only

Verification:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_task_manifest
- python3 scripts/task_manifest.py --help (script loads and parses)

Expected Evidence:
- Future task manifests show selected task artifacts with non-protected Files entries
- If a planning phase produces zero task files, the manifest includes a visible warning
- Preseed fallback tasks that would touch protected files are filtered out

Implementation Notes:
- This task was seeded by the harness before planner exploration. The assessment shows the codebase is currently healthy (1/1 and 2/2 strict verified in recent sessions), so the crisis premise is stale. Focus on the defensive improvements, not on fixing a live failure.
- Check whether Day 131's preseed improvements (task 2: "taught the preseed fallback to produce actionable, failure-specific tasks") already addressed the protected-file avoidance. If so, narrow this task to just the manifest visibility piece.
- Keep changes scoped to the three listed Python files. No Rust changes.
- If `PROTECTED_IMPLEMENTATION_FILES` doesn't exist as a named constant in preseed_session_plan.py, add one matching the list from evolve.sh safety rules: `scripts/evolve.sh`, `scripts/format_issues.py`, `scripts/build_site.py`, `.github/workflows/`, `IDENTITY.md`, `PERSONALITY.md`, `ECONOMICS.md`.
