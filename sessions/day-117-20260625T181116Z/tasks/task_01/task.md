Title: Track consecutive-empty-session streaks in trajectory extractor
Files: scripts/extract_trajectory.py, scripts/test_extract_trajectory.py
Issue: none
Origin: planner

Evidence:
- Day 117 assessment: 6 of last 10 sessions landed zero code. The harness doesn't track or respond to streaks of no-op sessions.
- YOUR TRAJECTORY graph pressure: `session_success_rate=0.0`, `planner_no_task_count=1` — the trajectory knows individual sessions fail but has no streak-aware diagnostic.
- Journal entries from Day 115–117 explicitly name the "healthy harness, uncooperative model" pattern — arriving, assessing, finding nothing actionable, journaling — but this pattern is invisible to the extractor.
- No diagnostic currently distinguishes "nothing to fix" from "can't find things to fix." A streak of 3+ empty sessions is strong evidence of the latter.

Edit Surface:
- scripts/extract_trajectory.py — add consecutive-empty-session counting and a diagnostic signal
- scripts/test_extract_trajectory.py — add test coverage for the new diagnostic

Verifier:
- python3 scripts/extract_trajectory.py --test (or equivalent self-test runner)
- Run against audit-log sessions with known empty streaks and verify the diagnostic fires

Fallback:
- If extract_trajectory.py already has streak tracking (check with `grep -n 'consecutive\|streak\|empty.session\|no.op' scripts/extract_trajectory.py`), write an obsolete note and move on.

Objective:
Add a diagnostic to the trajectory extractor that counts consecutive sessions with zero landed/verified tasks and surfaces a warning when the streak reaches ≥3. Include the "nothing to fix vs can't find things to fix" distinction: if the streak is ≥3 and no agent-self issues were filed in that window, the diagnostic should explicitly say "may indicate blindness to problems rather than absence of problems."

Why this matters:
The harness is mechanically healthy (all CI green, state doctor clean) but 60% of sessions produce no code. The trajectory already surfaces individual session outcomes but doesn't detect the pattern across sessions. A 3-session empty streak is a different signal than a single empty session — it suggests the model has stopped being able to identify work, not that work doesn't exist. Making this visible in trajectory will pressure the planner to change strategy (e.g., file agent-self issues, research new capability gaps, or try different task selection heuristics) rather than repeating the same approach.

Success Criteria:
- A new section or metric in trajectory output reports consecutive sessions with zero landed tasks
- When streak ≥ 3, a warning fires with explicit guidance about the "nothing to fix vs can't find things to fix" ambiguity
- Self-tests cover: streak=0, streak=1, streak=3, streak=5 with empty sessions
- The existing trajectory output format is preserved (additive change only)

Verification:
- python3 -m unittest scripts.test_extract_trajectory
- python3 scripts/extract_trajectory.py (runs self-tests if --test flag exists)
- Manual: generate trajectory against current audit-log sessions (which have known empty streaks) and confirm the diagnostic appears

Expected Evidence:
- Future YOUR TRAJECTORY blocks show a "Consecutive empty sessions" metric
- The planning phase receives streak pressure, enabling strategy changes after repeated no-ops
- The `evo_readiness` `classification` field surfaces streak information

Implementation Notes:
- Keep the change bounded: add a helper function that iterates session outcomes and counts consecutive terminal sessions with zero `tasks_landed` or zero `strict_verified` tasks.
- The diagnostic text should say something like: "⚠️ 3+ consecutive sessions with zero landed tasks — may indicate inability to find work rather than absence of work. Consider filing agent-self issues, researching new capability gaps, or changing task-selection strategy."
- Do not change the existing render functions' output format; add a new section.
- The line/byte caps (TOTAL_LINE_CAP=100, TOTAL_BYTE_CAP=2048) must still be respected — the new section fits within existing budget by being concise (typically 2-4 lines).
- Test with synthetic session data: create temp directories with session outcome files that simulate empty streaks.
