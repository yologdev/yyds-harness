Title: Harden seed contradiction detection with completion-language fallback
Files: scripts/preseed_session_plan.py
Issue: none
Origin: planner

Evidence:
- Trajectory shows task_seed_contradiction_count=1 from day-153 (2026-07-31 19:17:59) where one task had no_git_visible_changes — the implementation agent couldn't produce code for a seeded task, suggesting the seed was stale or contradicted by fresh assessment evidence.
- Log feedback corrected lesson: "seeded tasks contradicted the fresh assessment -> validate seeded tasks against fresh assessment evidence and replace contradicted seeds before implementation"
- Day 114 added completion-language detection to `check_task_contradiction()` but the vocabulary was narrow (matching only "fixed", "resolved", "shipped"). The assessment text uses richer completion language: "made landable", "given enough standalone weight", session-date prefixes like "Day 154 adjusted...", and qualitative closure phrases.
- The harness seed for this session produced task_01.md targeting lifecycle gaps — but Day 154 10:00 already handled the input-validation part (Task 1) and the model-call panic-path part (Task 2). A contradiction detector should have flagged that the seed was partially stale.
- Graph-derived pressure explicitly calls for: "Validate seeded tasks against fresh assessment (task_seed_contradiction_count=1)"

Edit Surface:
- scripts/preseed_session_plan.py — extend `check_task_contradiction()` with a broader completion-vocabulary fallback and a freshness check

Verifier:
- python3 -m pytest scripts/test_task_manifest.py -x -q 2>/dev/null || python3 -c "import scripts.preseed_session_plan as m; print('import OK')"

Fallback:
- If the assessment text format has changed and the completion-language patterns no longer match, or if the contradiction detector already covers all observed patterns, write a brief obsolete-task note. Do not add regex patterns that match assessment boilerplate (false positives are worse than false negatives).

Objective:
Make `check_task_contradiction()` in preseed_session_plan.py recognize when the fresh assessment describes a problem as already resolved, so stale seeds don't get sent to implementation agents.

Why this matters:
When a seed task targets a problem that the assessment already says is fixed, the implementation agent wastes turns discovering the fix is already in place. This produces no_git_visible_changes tasks that lower task success rate and waste API credits. The Day 114 fix added basic completion detection but the vocabulary gap means informal completion language (the kind assessments actually use) still slips through.

Success Criteria:
- `check_task_contradiction()` recognizes at least one new completion-language pattern that currently slips through (e.g., "Day NNN made this landable", "given enough standalone weight", "adjusted to trigger by itself").
- The existing contradiction tests continue to pass.
- No false positives: the detector does not flag genuinely open tasks as resolved.

Verification:
- python3 -m pytest scripts/test_task_manifest.py -x -q
- python3 -c "
from scripts.preseed_session_plan import check_task_contradiction
# Test: assessment says 'Day 154 made this landable'
result = check_task_contradiction('Close lifecycle gaps', 'Day 154 made this landable with the panic-path fix')
print('contradiction detected:', result)
"

Expected Evidence:
- Future trajectory shows lower task_seed_contradiction_count.
- Seeded tasks that target already-fixed problems are filtered out before reaching implementation.
- No new false-positive contradictions (tasks wrongly blocked).

Implementation Notes:
- The completion-vocabulary fallback should be a second pass after the existing keyword check: if the keyword check finds nothing, scan the assessment text for informal completion patterns like:
  - Session-date + completion verb: "Day NNN made/handled/fixed/resolved/closed/landed this"
  - Standalone-weight phrases: "given enough standalone weight", "adjusted to trigger by itself", "no longer needs"
  - Qualitative closure: "already addressed", "previously fixed", "the fix is in place", "this is done"
- Keep patterns specific enough to avoid matching boilerplate like "this assessment covers" or "this session will address."
- Add a freshness check: if the assessment's timestamp is newer than the seed's origin timestamp, and the assessment mentions the seed's topic in past-tense closure language, flag the contradiction.
- The existing `check_task_contradiction()` function is in preseed_session_plan.py — find it, understand its current logic, and extend it minimally.
- Run the existing test suite before and after to confirm no regressions.
