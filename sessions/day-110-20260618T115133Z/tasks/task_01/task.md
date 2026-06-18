Title: Add diagnostic output for state/transcript tool failure reconciliation gaps
Files: scripts/build_evolution_dashboard.py
Issue: none
Origin: planner

Evidence:
- Graph-derived next-task pressure row 1: "Reconcile transcript-only tool failures (transcript_only_failed_tool_count=2): Recent transcripts contained failed tool actions absent from state events."
- Graph-derived next-task pressure row 2: "Reconcile state-only tool failures (state_only_failed_tool_count=13): State events contained failed tool actions without matching transcript evidence."
- Trajectory snapshot: recent_action_evidence shows state_only_failed_tools=13, transcript_only_failed_tools=2
- The reconciliation metrics come from `unique_delta_count()` at line 2666-2667 of build_evolution_dashboard.py, computed from `state_failed_tools_all` vs `transcript_failed_tools_all`
- Currently the dashboard only reports the counts — it does not expose which specific failure labels are mismatched, making root-cause investigation impossible without manually reading both data sources

Edit Surface:
- scripts/build_evolution_dashboard.py (the reconciliation computation around lines 2635-2668)

Verifier:
- python3 scripts/build_evolution_dashboard.py --help (or equivalent dry-run)
- Verify the new diagnostic fields appear in the action_evidence output

Fallback:
- If the mismatch labels are empty (both lists are identical), the task is a no-op success — output the diagnostic anyway and note the gap has closed naturally
- If the dashboard script requires audit-log data that isn't available in the current environment, mark blocked with the exact missing data path

Objective:
Add a diagnostic artifact to the action_evidence section that lists the actual mismatched failure labels for both state-only and transcript-only tool failures, so future sessions can trace individual reconciliation gaps to specific tool calls.

Why this matters:
The harness has two event pipelines (state events + transcript logs) that disagree on 15 tool failures. Without knowing which specific failures are mismatched, root cause is invisible. A count alone ("13 state-only") doesn't tell you whether the gap is from timing skew, different failure classification thresholds, or missing event emission. Listing the actual labels makes the gap investigable in a single session.

Success Criteria:
- When state_only_failed_tool_count > 0, the action_evidence output includes a `state_only_failed_tool_labels` list showing the mismatched failure labels
- When transcript_only_failed_tool_count > 0, the action_evidence output includes a `transcript_only_failed_tool_labels` list showing the mismatched failure labels
- Existing reconciliation counts are preserved unchanged
- The new fields are compact (no more than 20 labels each, truncated if longer)

Verification:
- python3 -c "import scripts.build_evolution_dashboard" (syntax check)
- If possible: run a targeted test that feeds known mismatched tool lists and checks the diagnostic labels appear

Expected Evidence:
- Future trajectory snapshots include `state_only_failed_tool_labels` and `transcript_only_failed_tool_labels` in the action_evidence block
- The next `state summary` can cite specific mismatched failure labels instead of opaque counts
- A future session can trace one mismatched label to its root cause (timing, classification, or missing emission)

Implementation Notes:
- The `unique_delta_count` function at line 478 computes set difference counts. Add a companion that also returns the actual differing elements.
- At the reconciliation block (lines 2666-2667), compute the label lists alongside the counts.
- Store them in action_evidence as `state_only_failed_tool_labels` and `transcript_only_failed_tool_labels`.
- Truncate each list to 20 entries max to keep dashboard payloads compact.
- Do not change how failed tools are collected or classified — this task is purely diagnostic output.
- The implementation agent must not read large audit-log files; test with synthetic data.
