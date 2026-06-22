Title: Fix stale seed contradiction detection missing completed work in Recent Changes
Files: scripts/preseed_session_plan.py
Issue: none
Origin: planner

Evidence:
- The Day 114 12:45 session received a pre-seeded task (`task_01.md`) that was already completed in Day 114 08:48. The trajectory records `task_obsolete_count=1` and `obsolete_already_satisfied=1` for that session.
- The assessment's Recent Changes section says: "**Analysis-only task pressure made landable** (`scripts/preseed_session_plan.py`): The `task_no_edit_revert_count` metric was given enough standalone weight to trigger recovery tasks by itself..."
- This line contains the task key `task_no_edit_revert_count` but the contradiction detector (`_line_shows_resolution`) returned False because "made landable" doesn't match any resolution signal in `_RESOLUTION_SIGNALS`. The phrase "given enough standalone weight" is also a completion signal but unmatched.
- The pre-seeded task was passed to implementation with `validated_against_assessment: true`, causing the session to discover it was obsolete at implementation time.
- Trajectory graph pressure confirms: `task_manifest_seed_contradiction_count=1`.
- The log feedback lesson explicitly says: "seeded tasks contradicted the fresh assessment → validate seeded tasks against fresh assessment evidence and replace contradicted seeds before implementation."

Edit Surface:
- scripts/preseed_session_plan.py

Verifier:
- python3 scripts/preseed_session_plan.py --test

Fallback:
- If the preseed script's `--test` self-tests pass but the fix doesn't change behavior for the specific stale seed case, add a doctest or inline assertion that reproduces the exact assessment text from Day 114 08:48's Recent Changes entry and verify it flags the analysis-only task as contradicted.

Objective:
Make the task picker's contradiction detector recognize completed work even when the assessment describes it with phrases like "made landable" or "given enough standalone weight" rather than explicit "fixed" / "resolved" / "landed" signals.

Why this matters:
The harness wastes implementation sessions on tasks that were already shipped. When the assessment says a prior session completed the work, the task picker must detect that and either skip the seed or mark it contradicted so the implementation agent knows to stop early. This directly addresses the `task_obsolete_count` pressure in the trajectory and prevents the 0/1 verified sessions that drag down the task success rate.

Success Criteria:
- A preseed self-test reproduces the exact assessment text from Day 114 08:48's Recent Changes entry and confirms the "Make analysis-only task pressure landable" task is flagged as contradicted.
- Existing contradiction detection tests still pass (no regression in false-positive detection).
- The resolution signal list covers common assessment prose patterns for completed work: "made landable", "given enough standalone weight", "verified", "already in place".

Verification:
- python3 scripts/preseed_session_plan.py --test
- python3 -m unittest scripts.test_state_graph_tools

Expected Evidence:
- Future task manifests show `contradiction_reason` for seeds already completed in prior sessions.
- `task_manifest_seed_contradiction_count` drops to 0 when the assessment clearly describes prior-session completion.
- `task_obsolete_count` drops to 0 when no other cause exists.

Implementation Notes:
The contradiction detector lives in `_line_shows_resolution()` (line 411) and `check_task_contradiction()` (line 432). The `_RESOLUTION_SIGNALS` tuple (line 392) is a list of substrings that indicate a problem is already resolved. The current list misses patterns where the assessment says work was "made" into a desired state without using the past-tense resolution verbs.

Two complementary fixes:
1. **Expand resolution signals**: Add "made " (catches "made landable", "made deterministic"), "given enough standalone", "verified" (when describing past-session verification of a fix).
2. **Add a session-prefix gate in `_line_shows_resolution`**: When a line starts with a session-date prefix (regex: `Day \d+`), treat the line as describing already-completed work. If it also mentions a task key, it's resolution evidence. This catches cases where the assessment describes completed work without using any resolution-signal verb.

The self-tests in `--test` mode already have a contradiction test at line ~1064. Add a new test case that reproduces the exact stale-seed scenario: assessment text matching the Day 114 08:48 Recent Changes entry, and verifies the analysis-only task is contradicted.

Keep the change in `scripts/preseed_session_plan.py` only. If tests expose a dependency in `scripts/test_state_graph_tools.py`, include it but prefer the `--test` self-tests for verification.
