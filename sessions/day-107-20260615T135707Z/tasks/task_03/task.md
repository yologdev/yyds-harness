Title: Add trajectory freshness indicator to extract_trajectory.py
Files: scripts/extract_trajectory.py
Issue: none
Origin: planner

Objective:
Add a staleness warning to the YOUR TRAJECTORY block when the snapshot was computed more than one session ago, so the planner and implementation agents know when trajectory evidence may be stale.

Why this matters:
The assessment explicitly notes: "Trajectory warnings lag reality — the snapshot is computed at session start from audit-log, so it's always one session behind." This means pressure indicators like `deepseek_model_call_incomplete_count=7` may reference problems already fixed in the immediately prior session. When the planner sees a pressure indicator, it can't tell whether it's fresh or stale without cross-referencing the assessment.

A freshness indicator lets agents calibrate their trust in trajectory evidence: a 30-minute-old snapshot is reliable; a 6-hour-old snapshot covering multiple sessions may have stale claims.

Success Criteria:
- The YOUR TRAJECTORY block header includes a freshness line: e.g., `Snapshot age: 32m (1 session behind) — treat with caution` or `Snapshot age: 8m (fresh) — reliable`.
- When the snapshot is fresh (≤1 session behind), the indicator is reassuring (green/check style).
- When the snapshot is stale (>1 session behind), the indicator warns explicitly.
- The indicator does not break the existing 100-line / 2KB cap — freshness metadata is 1-2 lines.

Verification:
- python3 scripts/extract_trajectory.py (must exit 0 and produce valid markdown).
- Manual inspection: run the script and verify the freshness line appears in the YOUR TRAJECTORY header.
- The output must still fit within the existing size caps.

Expected Evidence:
- Future planning agents can cross-reference trajectory pressure with freshness to decide whether to act on a claim or defer until next session.
- The assessment's "Trajectory warnings lag reality" finding is partially addressed — not by eliminating lag (impossible without live audit-log), but by making it visible.

Implementation Notes:
- The script is `scripts/extract_trajectory.py` (~1800 lines).
- The freshness indicator should appear near the top of the YOUR TRAJECTORY block, after the "Last computed" line.
- To determine staleness: compare the snapshot computation time (already recorded as "Last computed: ...") against the most recent session's timestamp. If the most recent session in the audit-log is newer than the snapshot computation time, the snapshot is stale.
- Use the session timestamps from the audit-log directory names (e.g., `day-107-20260615T130431Z`).
- Keep the change minimal — add a helper function that computes freshness and format it as 1-2 lines in the trajectory header.
- Do not restructure the script or change existing behavior.
