Title: Add per-session non-proven claim detail to dashboard summary output
Files: scripts/build_evolution_dashboard.py
Issue: none
Origin: planner

Evidence:
- Trajectory claim health: 547/666 proven (82.1%); 119 non-proven (missing=89, observed=30); 2 recent non-proven
- Trajectory specifically: "recent non-proven claims: assessment_artifact=1 observed, run_lifecycle=1 missing"
- The `assessment_artifact` claim (observed, not proven) means an assessment artifact was observed but the claim about it couldn't be verified — likely a classification mismatch
- The `run_lifecycle` claim (missing) means a run lifecycle event claim is absent from the session's claims entirely — the claim wasn't even generated
- Currently the dashboard computes per-session claims at lines 4848-4914 but only reports aggregate status_counts and latest_unresolved — the per-session claim names and statuses are not surfaced in a way that lets the trajectory or planner trace which session produced which non-proven claim
- Without per-session claim detail, a planner can't tell whether the 2 recent non-proven claims are from the same session, different sessions, or whether they indicate a systemic gap vs a one-off

Edit Surface:
- scripts/build_evolution_dashboard.py (the claim summary computation around lines 4848-4914, and the session work_summary building around lines 2717-2750)

Verifier:
- python3 -c "import scripts.build_evolution_dashboard" (syntax check)
- If test fixtures exist: python3 -m pytest scripts/test_build_evolution_dashboard.py -x -k claim

Fallback:
- If the 2 recent non-proven claims have already resolved (all claims proven), mark this task success — add the diagnostic field anyway so future non-proven claims are traceable
- If claim data is only available in the full dashboard JSON and not in the trajectory extractor, the task should add the field to both or note the limitation

Objective:
Add a `non_proven_claim_sessions` field to the claim summary that maps each non-proven claim name to the session IDs where it appeared non-proven, so planners can trace which session produced which gap.

Why this matters:
119 non-proven claims is a large number, but without knowing which sessions they come from, it's impossible to distinguish "old sessions with known gaps" from "new sessions with fresh evidence pipeline failures." The 2 recent non-proven claims (assessment_artifact and run_lifecycle) are the most actionable because they're fresh — but the planner needs to know which session produced them to investigate. Adding session-level traceability converts "119 non-proven" from an opaque count into an investigable signal.

Success Criteria:
- The claim summary output includes a `non_proven_claim_sessions` field: a dict mapping claim_name → list of session_ids where that claim was non-proven
- Only the 10 most recent non-proven claim occurrences are included (to keep output bounded)
- Existing claim counts and status_counts are preserved unchanged
- The field is compact (no more than 5 session IDs per claim name)

Verification:
- python3 -c "import scripts.build_evolution_dashboard" (syntax check)
- If test fixtures allow: verify that a known non-proven claim appears with its session ID in the new field

Expected Evidence:
- Future trajectory snapshots include `non_proven_claim_sessions` in the claim summary
- The next planner can cite specific session IDs when investigating non-proven claims
- The `assessment_artifact` and `run_lifecycle` recent non-proven claims become traceable to their source sessions

Implementation Notes:
- The claim summary is built at lines ~4848-4914 in `build_evolution_dashboard.py`
- Each session has a `claims` list (list of dicts with name, status, etc.) and an `id` field
- The new field should be built by iterating sessions and collecting non-proven claims with their session IDs
- Limit: max 10 claim entries, max 5 session IDs per claim
- This is purely additive diagnostic output — no existing behavior should change
- The implementation agent should test with synthetic session data, not read audit-log archives
