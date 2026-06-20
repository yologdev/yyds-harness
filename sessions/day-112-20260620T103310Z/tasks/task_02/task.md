Title: Surface tool-name breakdown in state/transcript failure reconciliation
Files: scripts/build_evolution_dashboard.py
Issue: none
Origin: planner

Evidence:
- Trajectory graph-derived pressure row 2: "Reconcile transcript-only tool failures (transcript_only_failed_tool_count=5): Recent transcripts contained failed tool actions absent from state events"
- Trajectory graph-derived pressure row 3: "Reconcile state-only tool failures (state_only_failed_tool_count=17): State events contained failed tool actions without matching transcript entries"
- Assessment Finding #1 (MEDIUM): "State/transcript tool-failure reconciliation gap — 5 transcript-only and 17 state-only failed tool counts. The harness records tool failures in both the state event log and the transcript log, but they don't always agree."
- The dashboard already computes `state_only_failed_tool_labels` and `transcript_only_failed_tool_labels` at lines 2672–2673 of build_evolution_dashboard.py, but the HTML template (line ~7278) only renders the integer counts, not the label lists. The diagnostic data exists; it just isn't shown.

Edit Surface:
- scripts/build_evolution_dashboard.py — modify the HTML template to render state_only_failed_tool_labels and transcript_only_failed_tool_labels alongside the counts, so operators can see *which specific tools* diverge instead of only *how many*.

Verifier:
- python3 scripts/test_build_evolution_dashboard.py
- python3 -c "import scripts.build_evolution_dashboard" (import check)

Fallback:
- If the label lists are always empty in test data and cannot be populated with realistic fixtures, skip the HTML rendering change and instead add a note to the dashboard summary explaining that label-level reconciliation requires session-level audit-log data. Mark the task done with that documentation improvement.

Objective:
Make the dashboard HTML show *which tools* have state-only and transcript-only failure mismatches, not just integer counts. This turns an opaque count into an actionable diagnostic: operators can see whether the gap is concentrated in one tool type (e.g., bash) or spread across many.

Why this matters:
Day 110 taught the harness to name which tools fail instead of just counting them. This task extends that pattern from the `failed_tool_summary` section to the `action_evidence` reconciliation section. When the dashboard only shows "5 transcript-only failed tool actions," the operator has no way to know whether those are bash timeouts, search errors, or edit failures. Showing the labels turns a measurement gap into an actionable investigation lead.

Success Criteria:
- The dashboard HTML shows `state_only_failed_tool_labels` and `transcript_only_failed_tool_labels` when they are non-empty.
- If both lists are empty, the existing count display is unchanged (no new noise).
- The labels appear as a tooltip, expandable section, or compact inline text alongside the count — whichever fits the existing HTML style best.
- Existing tests pass without modification (the data structure isn't changing, only the rendering).

Verification:
- python3 scripts/test_build_evolution_dashboard.py
- Visual check: generate dashboard HTML with test fixtures and verify the tool-label breakdown appears in the action evidence section.

Expected Evidence:
- Future dashboard HTML shows tool-name-level reconciliation data in the per-session action evidence row.
- The `state_only_failed_tool_count` and `transcript_only_failed_tool_count` gnomes remain unchanged; only their display context improves.
- Operators can distinguish "bash failures missing from state" from "edit failures missing from transcripts" without manual log inspection.

Implementation Notes:
- The change is in the JavaScript template string around line ~7278 of build_evolution_dashboard.py. Look for `stateOnlyFails` and `transcriptOnlyFails` in the HTML template.
- The `state_only_failed_tool_labels` and `transcript_only_failed_tool_labels` are already populated by the `action_evidence` dict at lines 2672–2673. They are compact lists (max 20 items from `unique_delta_labels`). They land in the JSON data that the HTML reads as `evidence.state_only_failed_tool_labels` and `evidence.transcript_only_failed_tool_labels`.
- The render pattern should follow existing dashboard conventions: use `text()` for safe escaping, show labels as comma-separated inline text or a `<details>` expandable section.
- Do not change the data pipeline or add new gnomes. This is purely a rendering improvement that makes existing computed data visible.
- The task builds directly on Day 110's `failed_tool_pattern_summary` approach — same idea (name the tools), different dashboard section (reconciliation vs. failure summary).
