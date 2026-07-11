Title: Improve stale-seed contradiction detection in preseed task picker
Files: scripts/preseed_session_plan.py
Issue: none
Origin: planner
validated_against_assessment: true

Evidence:
- Day 133 04:55 session had `obsolete_already_satisfied=1`: the seed task was
  marked obsolete because the assessment showed the problem was already resolved.
  The contradiction detector should have caught this before seeding but didn't.
- Day 133 04:41 session had `reverted_no_edit=1` and `reverted_unlanded_source_edits=1`:
  tasks were selected but couldn't produce overlapping file changes.
- The assessment says: "Day 133 (2026-07-11 11:28:00): tasks 2/2" — the harness
  recovered, but the preseed picker wasted a task slot on an obsolete seed.
- Day 118 learning: "Shared evidence is not shared understanding when subsystems
  parse through different dictionaries" — the contradiction detector in
  `check_task_contradiction` uses fixed vocabulary that may not match how the
  assessment describes completed or resolved work.
- The recent trajectory shows 2 `reverted_no_edit` tasks across sessions: tasks
  that got selected but produced zero code changes. Stronger contradiction
  detection would have flagged these as stale and not seeded them.

Edit Surface:
- scripts/preseed_session_plan.py

Verifier:
- python3 scripts/preseed_session_plan.py --test
- python3 -c "import scripts.preseed_session_plan as p; print('import OK')"

Fallback:
- If the current assessment shows no obsolete-seed or reverted-no-edit patterns
  in the last 3 sessions, write session_plan/task_01_obsolete.md with the
  evidence that this failure class is no longer live.

Objective:
Make the preseed task picker's contradiction detector recognize when the
assessment resolves or obsoletes a seed task, so implementation agents don't
waste slots on already-resolved work.

Why this matters:
When the preseed picker seeds a task the assessment already shows is done, that
task either (a) gets rejected as obsolete (wasting a slot), or (b) gets
implemented as no-op work that reverts because it touches no source files.
Either way, it's wasted evolution capacity. Better contradiction detection
directly improves task_success_rate (a fitness gnome) by preventing stale seeds
from consuming task slots.

Success Criteria:
- `python3 scripts/preseed_session_plan.py --test` passes.
- The `check_task_contradiction` function detects at least these resolution
  patterns in assessment text: "already satisfied", "already done", "resolved",
  "no longer needed", "obsolete", and session-date prefixed resolution language
  like "Day NNN made this landable" or "Day NNN resolved this".
- Tasks that contradict the assessment are not seeded (or are seeded with a
  lower priority that lets evidence-backed tasks take precedence).

Verification:
- python3 scripts/preseed_session_plan.py --test
- Manual check: run the preseed script against the current assessment and verify
  it doesn't seed tasks the assessment already marks as resolved.

Expected Evidence:
- Future task state counts show fewer `obsolete_already_satisfied` tasks.
- The `task_spec_quality_score` gnome rises as fewer stale seeds reach
  implementation.
- The trajectory shows fewer `reverted_no_edit` entries from seeded tasks that
  couldn't produce code changes.

Implementation Notes:
- Focus on `check_task_contradiction()` in scripts/preseed_session_plan.py (around
  line 195-260).
- The Day 118 learning about "different dictionaries" is the root cause:
  the assessment writes "already satisfied" or "Day NNN made this landable"
  but the contradiction detector looks for structured metric keys.
- Add a semantic fallback: scan the assessment text for resolution-language
  patterns near the task title or task key words.
- Keep the change scoped to this one file. If the fix requires changes to how
  the assessment is structured, write a note about the dependency and scope
  down to just the detection improvement.
- Study the existing `CONTRADICTION_PATTERNS` or equivalent data structure and
  add the missing resolution phrases.
- The self-test in the file should exercise the new detection patterns.
