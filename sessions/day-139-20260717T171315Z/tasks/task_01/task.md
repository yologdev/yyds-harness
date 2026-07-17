Title: Add held-out eval fixture for FailureObserved dedup lifecycle scenario
Files: eval/fixtures/local-smoke/372-state-lifecycle-dedup-failure-observed.json
Issue: none
Origin: planner

Evidence:
- Day 139 morning session landed a +61 line unit test in test_append_terminal_state_events.py
  that validates deduplication of retroactive FailureObserved events (commit b45050f2).
- The production code path in scripts/append_terminal_state_events.py already handles
  open_after_FailureObserved runs (lines 230, 280-281, 514-515).
- Capability fitness score=1.0, recommendation: "add held-out coding eval evidence."
- No eval fixture exists for this lifecycle scenario (highest lifecycle fixture is
  371-state-lifecycle-pairing.json; no dedup-specific fixture).
- The graph pressure shows state_run_unmatched_non_validation_completed_count=2 from
  open_after_FailureObserved causes — these are pre-existing orphans that the janitor
  script will close, but an eval fixture proves the dedup logic works.

Edit Surface:
- eval/fixtures/local-smoke/372-state-lifecycle-dedup-failure-observed.json

Verifier:
- python3 -c "import json; json.load(open('eval/fixtures/local-smoke/372-state-lifecycle-dedup-failure-observed.json'))" && echo "valid JSON"
- Verify the fixture includes expected claims: at least one assertion about FailureObserved dedup behavior

Fallback:
- If the fixture number 372 is already taken, use the next available number.
- If the append script's dedup logic has changed since the morning session, adapt the fixture
  to match current behavior rather than forcing a script change.

Objective:
Add a held-out eval fixture that validates the FailureObserved deduplication logic in
scripts/append_terminal_state_events.py, giving the dashboard a concrete data point
for the lifecycle dedup scenario.

Why this matters:
The morning session's fix was test-only — it validated the production code but didn't
add eval coverage. Eval fixtures are how the dashboard measures harness quality across
sessions. Adding one for this scenario:
- Proves the fix works at the eval level, not just the unit-test level
- Gives the dashboard a new data point to track lifecycle health
- Addresses the fitness recommendation to "add held-out coding eval evidence"
- Provides a regression guard: if dedup breaks in the future, the eval catches it

Success Criteria:
- Valid JSON fixture at eval/fixtures/local-smoke/372-state-lifecycle-dedup-failure-observed.json
- Fixture includes assertions about FailureObserved event deduplication behavior
- Fixture follows the conventions of existing eval fixtures (see 371-state-lifecycle-pairing.json
  for the lifecycle fixture pattern)

Verification:
- python3 -c "import json; json.load(open('eval/fixtures/local-smoke/372-state-lifecycle-dedup-failure-observed.json'))"
- Confirm the fixture file exists and is valid JSON

Expected Evidence:
- Dashboard eval summary picks up the new fixture
- Future state lifecycle graphs have one more held-out validation point
- The 2 historical open_after_FailureObserved runs remain as pre-existing orphans
  (the eval fixture validates the logic, not the historical data)

Implementation Notes:
- Study 371-state-lifecycle-pairing.json for the lifecycle fixture pattern (claims structure,
  expected/actual assertions, metadata conventions)
- The fixture should assert: when the janitor script encounters a run with FailureObserved
  but no RunCompleted, it emits exactly one RunCompleted (not duplicates from multiple
  script invocations).
- This mirrors the unit test added in commit b45050f2 but at the eval-fixture level.
- Keep the fixture small — 30-60 lines of JSON, focused on one scenario.
