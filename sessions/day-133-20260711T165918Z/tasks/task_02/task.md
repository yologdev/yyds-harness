Title: Broaden verification gate to accept issue-management and non-code tasks
Files: scripts/task_verification_gate.py
Issue: #93
Origin: planner
validated_against_assessment: true

Evidence:
- Issue #93 ("Close resolved issues #89, #91, #92") keeps reverting because the
  verification gate rejects it with "task changes do not overlap planned Files
  entries". The task has Files: "(none — GitHub issue management only, no source
  edits)".
- The root cause is `external_only_planned()` in scripts/task_verification_gate.py
  line 38-42: it only recognizes planned files entries that contain both "none"
  AND "gh cli". The #93 task's planned files entry says "none — GitHub issue
  management only, no source edits" which contains "GitHub" but not "gh cli".
- Because `external_only_planned` returns False, the verification never reaches
  the external-evidence check. The task produces a valid `task_01_external_evidence.json`
  with gh CLI output but the gate never evaluates it.
- The gate DOES work for tasks that use the exact phrase "none (gh CLI only)"
  (see the self-test on line 214), proving the mechanism is correct but the
  recognition pattern is too narrow.
- The assessment notes this as a MEDIUM friction: "The verifier doesn't have a
  path for non-code tasks."

Edit Surface:
- scripts/task_verification_gate.py

Verifier:
- python3 scripts/task_verification_gate.py --test

Fallback:
- If the test already passes with broader patterns or the function has already
  been updated, verify by running the test and mark the task as satisfied.
- If broadening the pattern would accept too many false positives, note the
  trade-off and keep the change minimal (one additional pattern).

Objective:
Make the verification gate accept non-code tasks that produce valid external
evidence (gh CLI output, API responses, issue management), so tasks like closing
stale GitHub issues can pass verification without touching source files.

Why this matters:
The verification gate was designed to prevent scope creep — tasks that change
files they didn't plan to change. But it currently blocks legitimate non-code
work. This causes:
1. Issue #93 keeps reverting, wasting implementation slots on a task that can
   never pass.
2. The open issue backlog accumulates noise (issues #89, #91, #92 describe
   already-completed work but stay OPEN).
3. Future non-code tasks (release tagging, GitHub API operations, discussion
   responses) will hit the same wall.
4. The `task_verification_rate` gnome is depressed by tasks that can never pass
   verification regardless of quality.

Success Criteria:
- `python3 scripts/task_verification_gate.py --test` passes (existing + new tests).
- `external_only_planned()` returns True for planned file entries that describe
  non-code work, including at minimum these patterns:
  - "none (gh CLI only)" — already works
  - "none — GitHub issue management only, no source edits" — currently fails
  - "none" followed by any description indicating no source file edits
- The verification gate's `ok` is True for a task with non-code planned files
  AND valid external evidence (status "completed" with non-empty evidence).

Verification:
- python3 scripts/task_verification_gate.py --test

Expected Evidence:
- Issue #93 stops reverting when re-attempted — the verification gate accepts
  the external evidence from gh CLI operations.
- The `task_verification_rate` gnome rises as non-code tasks can now pass.
- Future task state counts show fewer `reverted_scope_mismatch` entries for
  issue-management tasks.

Implementation Notes:
- The fix is in `external_only_planned()` (line 38-42). Currently:
  ```python
  def external_only_planned(planned: list[str]) -> bool:
      if not planned:
          return False
      normalized = [" ".join(str(item).lower().split()) for item in planned]
      return all(item.startswith("none") and "gh cli" in item for item in normalized)
  ```
- The problem: `"gh cli" in item` is too strict. It doesn't match "GitHub issue
  management only" or "issue management only" or other descriptions of non-code work.
- Fix options (choose the simplest that works):
  a) Broaden "gh cli" to also match "github" — catches most variants but may
     have false positives (e.g., "none — update GitHub README").
  b) Check for "none" prefix AND absence of file extensions or paths — more
     robust but more complex.
  c) Add a secondary check: if the planned entry starts with "none" AND contains
     any of {"github", "gh cli", "issue management", "no source edits", "no code"},
     treat it as external-only.
- Option (c) is the best balance: it's explicit about what counts as non-code
  work without being so broad it catches "none — update src/main.rs".
- Update the self-test (line 214) to also test the broader patterns.
- The `valid_external_evidence()` function (line 58-63) already works correctly
  — it checks for status "completed"/"changed" with non-empty evidence. No
  change needed there.
